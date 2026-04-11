//! 代码视图渲染：行号栏 + 语法高亮 + 搜索匹配
//!
//! 提供只读和可编辑两种 TextEdit 视图，
//! 通过 layouter 注入语法高亮和搜索匹配高亮。
//! 行号栏通过 painter overlay 绘制在视口左侧固定位置。
//!
//! @author sky

use crate::highlight::{self, Language, Span};
use crate::search::FindMatch;
use crate::theme::CodeViewTheme;
use crate::viewport;
pub use crate::viewport::VIEWPORT_TEXT_LEN;
use eframe::egui;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// 只读视图布局缓存
///
/// 文本和 span 在只读模式下不变，仅搜索状态变化触发重建。
pub struct LayoutCache {
    /// 搜索匹配数
    match_count: usize,
    /// 当前高亮的匹配索引
    current_match: Option<usize>,
    /// 缓存的 galley
    galley: Arc<egui::Galley>,
}

/// 大文本 debounce 阈值：超过此长度的文本在编辑时延迟重排版
const DEBOUNCE_TEXT_LEN: usize = 100_000;
/// debounce 等待时间（秒）
const DEBOUNCE_SECS: f64 = 0.3;

/// 可编辑视图布局缓存
///
/// 通过文本哈希检测内容变更，缓存 tree-sitter span 和 galley。
pub struct EditableLayoutCache {
    /// 文本内容哈希
    pub(crate) text_hash: u64,
    /// 缓存的语法高亮 span
    pub(crate) spans: Vec<Span>,
    /// 搜索匹配数
    pub(crate) match_count: usize,
    /// 当前高亮的匹配索引
    pub(crate) current_match: Option<usize>,
    /// 缓存的 galley
    pub(crate) galley: Arc<egui::Galley>,
    /// 大文本编辑 debounce：galley 过期但尚未重排版
    pub(crate) stale: bool,
    /// 文本最后一次变更时间
    pub(crate) last_change_time: f64,
    /// 是否启用视窗模式（大文本）
    pub(crate) is_viewport: bool,
    /// 视窗模式：光标在全文中的字符偏移
    pub(crate) viewport_cursor_char: usize,
    /// 视窗模式：当前窗口起始字符偏移
    pub(crate) viewport_window_start: usize,
    /// 全文字符数缓存（视窗模式用，避免每帧 O(n)）
    pub(crate) full_text_chars: usize,
    /// 全文行数缓存（视窗模式用，避免每帧 O(n)）
    pub(crate) full_text_lines: usize,
    /// 全文字节数缓存（用于检测文本变更后刷新 chars/lines）
    pub(crate) full_text_len: usize,
}

impl EditableLayoutCache {
    /// 当前是否处于视窗模式
    pub fn is_viewport(&self) -> bool {
        self.is_viewport
    }
}

/// 计算文本内容哈希（SipHash，660KB ~0.3ms）
pub(crate) fn hash_text(text: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// 字符偏移转字节偏移
pub(crate) fn byte_offset_at_char(s: &str, char_offset: usize) -> usize {
    s.char_indices()
        .nth(char_offset)
        .map_or(s.len(), |(i, _)| i)
}

/// 行号栏右侧到文本的间距
pub(crate) const GUTTER_PAD: f32 = 8.0;
/// 行内左侧 padding
pub(crate) const TEXT_PAD_LEFT: f32 = 8.0;

/// 行号栏宽度（根据最大行号计算位数）
pub fn line_number_width(max_number: usize) -> f32 {
    let digits = if max_number == 0 {
        1
    } else {
        (max_number as f32).log10().floor() as usize + 1
    };
    digits as f32 * 8.0 + 24.0
}

/// 在 ScrollArea 外绘制全高背景（左侧 gutter + 右侧编辑区）
pub fn paint_editor_bg(ui: &egui::Ui, full_rect: egui::Rect, gutter_w: f32, theme: &CodeViewTheme) {
    let painter = ui.painter();
    painter.rect_filled(full_rect, 0.0, theme.bg);
    painter.rect_filled(
        egui::Rect::from_min_size(
            full_rect.left_top(),
            egui::vec2(gutter_w + GUTTER_PAD, full_rect.height()),
        ),
        0.0,
        theme.gutter_bg,
    );
}

/// 重建语法高亮 galley（提取公共逻辑，被普通模式和视窗模式共用）
///
/// 检查缓存命中后执行完整重建：tree-sitter 解析 → LayoutJob → 字体布局。
pub(crate) fn rebuild_galley(
    ui: &egui::Ui,
    text: &str,
    text_hash: u64,
    lang: Language,
    match_ranges: &[(usize, usize)],
    current_match: Option<usize>,
    theme: &CodeViewTheme,
    cache: &mut Option<EditableLayoutCache>,
    is_viewport: bool,
    viewport_cursor_char: usize,
    viewport_window_start: usize,
) -> Option<Arc<egui::Galley>> {
    let mc = match_ranges.len();
    // 缓存命中
    if let Some(c) = cache.as_ref() {
        if c.text_hash == text_hash
            && !c.stale
            && c.match_count == mc
            && c.current_match == current_match
        {
            return Some(c.galley.clone());
        }
    }
    // 全量重建
    let old = cache.take();
    let text_changed = old.as_ref().map_or(true, |c| c.text_hash != text_hash);
    let spans = if text_changed {
        highlight::compute_spans(text, lang)
    } else {
        old.unwrap().spans
    };
    let job =
        highlight::build_layout_job_with_matches(text, &spans, match_ranges, current_match, theme);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    *cache = Some(EditableLayoutCache {
        text_hash,
        spans,
        match_count: mc,
        current_match,
        galley: galley.clone(),
        stale: false,
        last_change_time: 0.0,
        is_viewport,
        viewport_cursor_char,
        viewport_window_start,
        full_text_chars: 0,
        full_text_lines: 0,
        full_text_len: 0,
    });
    Some(galley)
}

/// 只读代码视图（TextEdit + 语法高亮 + 搜索高亮 + 行号）
///
/// `line_mapping` 为空时显示顺序行号（1-indexed），
/// 非空时显示原始源码行号，无映射行不显示行号。
pub fn code_view(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &str,
    spans: &[Span],
    line_mapping: &[Option<u32>],
    matches: &[FindMatch],
    current: Option<usize>,
    theme: &CodeViewTheme,
    cache: &mut Option<LayoutCache>,
    scroll_to_line: &mut Option<usize>,
) {
    let line_count = text.split('\n').count().max(1);
    let max_number = if line_mapping.is_empty() {
        line_count
    } else {
        line_mapping
            .iter()
            .filter_map(|n| n.map(|v| v as usize))
            .max()
            .unwrap_or(line_count)
    };
    let gutter_w = line_number_width(max_number);
    let code_font = egui::FontId::monospace(theme.code_font_size);
    let match_ranges: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    let match_ref = match_ranges.as_slice();
    // layouter: 语法高亮 + 搜索匹配背景（带缓存）
    let mut layouter =
        |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| -> Arc<egui::Galley> {
            let mc = match_ref.len();
            let cm = current;
            if let Some(c) = cache.as_ref() {
                if c.match_count == mc && c.current_match == cm {
                    return c.galley.clone();
                }
            }
            let s = text_buf.as_str();
            let job = highlight::build_layout_job_with_matches(s, spans, match_ref, cm, theme);
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            *cache = Some(LayoutCache {
                match_count: mc,
                current_match: cm,
                galley: galley.clone(),
            });
            galley
        };
    let mut galley_y = 0.0f32;
    let mut x_origin = 0.0f32;
    let mut edge_scroll_delta = 0.0f32;
    let mut wheel_delta = 0.0f32;
    // 滚动到指定行时记录触发时间
    let hl_time_id = id.with("__hl_time");
    let hl_line_id = id.with("__hl_line");
    if scroll_to_line.is_some() {
        let line = scroll_to_line.unwrap();
        let now = ui.input(|i| i.time);
        ui.ctx().data_mut(|d| {
            d.insert_temp(hl_time_id, now);
            d.insert_temp(hl_line_id, line);
        });
    }
    ui.horizontal_top(|ui| {
        x_origin = ui.cursor().left();
        ui.add_space(gutter_w + GUTTER_PAD + TEXT_PAD_LEFT);
        const HOLD: f64 = 0.5;
        const FADE: f64 = 0.8;
        let hl_time: Option<f64> = ui.ctx().data(|d| d.get_temp(hl_time_id));
        if let Some(start) = hl_time {
            let elapsed = ui.input(|i| i.time) - start;
            let alpha = if elapsed < HOLD {
                1.0
            } else {
                (1.0 - ((elapsed - HOLD) / FADE)).max(0.0)
            };
            if alpha > 0.001 {
                let hl_line: usize = ui.ctx().data(|d| d.get_temp(hl_line_id).unwrap_or(0));
                let row_h = ui.fonts_mut(|f| {
                    f.layout_no_wrap("M".into(), code_font.clone(), egui::Color32::WHITE)
                        .size()
                        .y
                });
                let top_y = ui.min_rect().top();
                let y = top_y + hl_line as f32 * row_h;
                let full_width = ui.max_rect().width();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(ui.max_rect().left(), y),
                    egui::vec2(full_width, row_h),
                );
                let [r, g, b, _] = theme.line_highlight.to_array();
                let a = (alpha as f32 * 255.0) as u8;
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );
                ui.ctx().request_repaint_after_secs(0.016);
            } else {
                ui.ctx().data_mut(|d| {
                    d.remove_temp::<f64>(hl_time_id);
                    d.remove_temp::<usize>(hl_line_id);
                });
            }
        }
        let mut buf: &str = text;
        let output = code_text_edit(&mut buf, id, code_font.clone(), &mut layouter).show(ui);
        galley_y = output.galley_pos.y;
        // 滚动到指定行
        if let Some(line) = scroll_to_line.take() {
            let row_h = output.galley.size().y / line_count.max(1) as f32;
            let target_y = galley_y + line as f32 * row_h;
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(output.galley_pos.x, target_y),
                egui::vec2(1.0, row_h),
            );
            ui.scroll_to_rect(line_rect, Some(egui::Align::Center));
        }
        if output.response.dragged() {
            let (ed, wd) = detect_edge_scroll(&output.response, ui);
            edge_scroll_delta = ed;
            wheel_delta = wd;
        }
    });
    apply_scroll_delta(ui, edge_scroll_delta, wheel_delta);
    paint_line_numbers(
        ui,
        galley_y,
        line_count,
        line_mapping,
        gutter_w,
        &code_font,
        theme,
        x_origin,
    );
}

/// 可编辑代码视图（TextEdit + 语法高亮 + 搜索高亮 + 行号）
///
/// 返回 `true` 表示文本已被修改，调用方負责刷新高亮数据和标记 tab 状态。
pub fn code_view_editable(
    ui: &mut egui::Ui,
    id: egui::Id,
    text: &mut String,
    lang: Language,
    matches: &[FindMatch],
    current: Option<usize>,
    theme: &CodeViewTheme,
    cache: &mut Option<EditableLayoutCache>,
    viewport_override: Option<bool>,
    scroll_to_line: &mut Option<usize>,
) -> bool {
    // 视窗模式判断：优先用手动覆盖，否则自动检测
    let is_viewport = match viewport_override {
        Some(v) => v,
        None => match cache.as_ref() {
            Some(c) => c.is_viewport,
            None => text.len() > viewport::VIEWPORT_TEXT_LEN,
        },
    };
    // 过渡帧：从视窗切换到普通模式时，先渲染一帧 loading overlay 再实际切换，
    // 避免用户感受到 galley 全量布局的卡顿。
    if !is_viewport {
        if let Some(c) = cache.as_ref() {
            if c.is_viewport {
                viewport::code_view_editable_viewport(
                    ui,
                    id,
                    text,
                    lang,
                    matches,
                    current,
                    theme,
                    cache,
                    scroll_to_line,
                );
                // viewport 函数内部会把 is_viewport 写回 true，必须在之后强制覆盖
                if let Some(c) = cache.as_mut() {
                    c.is_viewport = false;
                }
                paint_transition_overlay(ui, theme);
                ui.ctx().request_repaint();
                return false;
            }
        }
    }
    if is_viewport {
        return viewport::code_view_editable_viewport(
            ui,
            id,
            text,
            lang,
            matches,
            current,
            theme,
            cache,
            scroll_to_line,
        );
    }
    let line_count = text.split('\n').count().max(1);
    let gutter_w = line_number_width(line_count);
    let code_font = egui::FontId::monospace(theme.code_font_size);
    let match_ranges: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    let match_ref = match_ranges.as_slice();
    // layouter: 语法高亮 + 搜索匹配背景（带缓存，避免每帧重建 LayoutJob 和重跑 tree-sitter）
    let mut layouter =
        |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| -> Arc<egui::Galley> {
            let s = text_buf.as_str();
            let th = hash_text(s);
            // 大文本 debounce：编辑时用纯文本 galley（跟手），停止输入后再做完整高亮
            if s.len() > DEBOUNCE_TEXT_LEN {
                if let Some(c) = cache.as_mut() {
                    let now = ui.input(|i| i.time);
                    if c.text_hash != th {
                        // 文本变更：立即用纯文本 LayoutJob 重排版（跳过 tree-sitter）
                        c.text_hash = th;
                        c.stale = true;
                        c.last_change_time = now;
                        let plain_color = theme.syntax.text;
                        let job = egui::text::LayoutJob::simple(
                            s.to_owned(),
                            code_font.clone(),
                            plain_color,
                            f32::INFINITY,
                        );
                        let galley = ui.fonts_mut(|f| f.layout_job(job));
                        c.galley = galley.clone();
                        ui.ctx().request_repaint_after_secs(DEBOUNCE_SECS as f32);
                        return galley;
                    }
                    if c.stale && now - c.last_change_time < DEBOUNCE_SECS {
                        // debounce 窗口内，复用上次的纯文本 galley
                        let remaining = DEBOUNCE_SECS - (now - c.last_change_time) + 0.016;
                        ui.ctx().request_repaint_after_secs(remaining as f32);
                        return c.galley.clone();
                    }
                    c.stale = false;
                }
            }
            rebuild_galley(
                ui, s, th, lang, match_ref, current, theme, cache, false, 0, 0,
            )
            .unwrap()
        };
    let mut galley_y = 0.0f32;
    let mut x_origin = 0.0f32;
    let mut edge_scroll_delta = 0.0f32;
    let mut wheel_delta = 0.0f32;
    let mut changed = false;
    // 滚动到指定行时记录触发时间
    let hl_time_id = id.with("__hl_time");
    let hl_line_id = id.with("__hl_line");
    if scroll_to_line.is_some() {
        let line = scroll_to_line.unwrap();
        let now = ui.input(|i| i.time);
        ui.ctx().data_mut(|d| {
            d.insert_temp(hl_time_id, now);
            d.insert_temp(hl_line_id, line);
        });
    }
    ui.horizontal_top(|ui| {
        x_origin = ui.cursor().left();
        ui.add_space(gutter_w + GUTTER_PAD + TEXT_PAD_LEFT);
        // 行高亮动画（TextEdit 之前绘制，避免遮盖文字）
        const HOLD: f64 = 0.5;
        const FADE: f64 = 0.8;
        let hl_time: Option<f64> = ui.ctx().data(|d| d.get_temp(hl_time_id));
        if let Some(start) = hl_time {
            let elapsed = ui.input(|i| i.time) - start;
            let alpha = if elapsed < HOLD {
                1.0
            } else {
                (1.0 - ((elapsed - HOLD) / FADE)).max(0.0)
            };
            if alpha > 0.001 {
                let hl_line: usize = ui.ctx().data(|d| d.get_temp(hl_line_id).unwrap_or(0));
                let row_h = ui.fonts_mut(|f| {
                    f.layout_no_wrap("M".into(), code_font.clone(), egui::Color32::WHITE)
                        .size()
                        .y
                });
                let top_y = ui.min_rect().top();
                let y = top_y + hl_line as f32 * row_h;
                let full_width = ui.max_rect().width();
                let rect = egui::Rect::from_min_size(
                    egui::pos2(ui.max_rect().left(), y),
                    egui::vec2(full_width, row_h),
                );
                let [r, g, b, _] = theme.line_highlight.to_array();
                let a = (alpha as f32 * 255.0) as u8;
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                );
                ui.ctx().request_repaint_after_secs(0.016);
            } else {
                ui.ctx().data_mut(|d| {
                    d.remove_temp::<f64>(hl_time_id);
                    d.remove_temp::<usize>(hl_line_id);
                });
            }
        }
        let output = code_text_edit(text, id, code_font.clone(), &mut layouter).show(ui);
        galley_y = output.galley_pos.y;
        changed = output.response.changed();
        // 滚动到指定行
        if let Some(line) = scroll_to_line.take() {
            let row_h = output.galley.size().y / line_count.max(1) as f32;
            let target_y = galley_y + line as f32 * row_h;
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(output.galley_pos.x, target_y),
                egui::vec2(1.0, row_h),
            );
            ui.scroll_to_rect(line_rect, Some(egui::Align::Center));
        }
        if output.response.dragged() {
            let (ed, wd) = detect_edge_scroll(&output.response, ui);
            edge_scroll_delta = ed;
            wheel_delta = wd;
        }
    });
    apply_scroll_delta(ui, edge_scroll_delta, wheel_delta);
    let line_count_now = text.split('\n').count().max(1);
    paint_line_numbers(
        ui,
        galley_y,
        line_count_now,
        &[],
        gutter_w,
        &code_font,
        theme,
        x_origin,
    );
    changed
}

/// 行号 overlay
///
/// `x_origin` 是行号区域的左边缘绝对 X 坐标（从 `horizontal_top` 内部捕获），
/// 不依赖 `clip_rect().left()` 以兼容嵌套 ScrollArea 等场景。
pub(crate) fn paint_line_numbers(
    ui: &egui::Ui,
    galley_y: f32,
    line_count: usize,
    line_mapping: &[Option<u32>],
    gutter_w: f32,
    font: &egui::FontId,
    theme: &CodeViewTheme,
    x_origin: f32,
) {
    let clip = ui.clip_rect();
    let painter = ui.painter();
    let measure = painter.layout_no_wrap("M".to_string(), font.clone(), egui::Color32::WHITE);
    let line_height = measure.size().y;
    // 行号区背景（仅覆盖实际行区域，不扩展到整个 clip 高度；
    //            全高背景由调用方通过 paint_editor_bg 负责）
    let content_bottom = galley_y + line_count as f32 * line_height;
    let gutter_top = galley_y.max(clip.top());
    let gutter_bottom = content_bottom.min(clip.bottom());
    if gutter_bottom > gutter_top {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x_origin, gutter_top),
                egui::pos2(x_origin + gutter_w + GUTTER_PAD, gutter_bottom),
            ),
            0.0,
            theme.gutter_bg,
        );
    }
    let gutter_right_x = x_origin + gutter_w;
    let first = ((clip.top() - galley_y) / line_height).max(0.0) as usize;
    let last = ((clip.bottom() - galley_y) / line_height + 1.0)
        .ceil()
        .min(line_count as f32) as usize;
    for i in first..last {
        let y = galley_y + i as f32 * line_height;
        let line_label: Option<usize> = if line_mapping.is_empty() {
            Some(i + 1)
        } else {
            line_mapping.get(i).and_then(|n| n.map(|v| v as usize))
        };
        if let Some(num) = line_label {
            painter.text(
                egui::pos2(gutter_right_x - 8.0, y),
                egui::Align2::RIGHT_TOP,
                num,
                font.clone(),
                theme.line_number_color,
            );
        }
    }
}

/// 检测拖拽选择时的边缘滚动和滚轮事件
pub(crate) fn detect_edge_scroll(response: &egui::Response, ui: &egui::Ui) -> (f32, f32) {
    let clip = ui.clip_rect();
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let edge_zone = 30.0;
    let max_speed = 800.0;
    let mut edge_delta = 0.0f32;
    // 鼠标靠近或超出视口边缘时自动滚动
    if let Some(pos) = response.interact_pointer_pos() {
        let speed = if pos.y < clip.top() {
            let dist = clip.top() - pos.y + edge_zone;
            (dist / edge_zone).min(3.0) * max_speed
        } else if pos.y > clip.bottom() {
            let dist = pos.y - clip.bottom() + edge_zone;
            -((dist / edge_zone).min(3.0) * max_speed)
        } else if pos.y < clip.top() + edge_zone {
            let factor = (clip.top() + edge_zone - pos.y) / edge_zone;
            factor * max_speed * 0.3
        } else if pos.y > clip.bottom() - edge_zone {
            let factor = (pos.y - clip.bottom() + edge_zone) / edge_zone;
            -(factor * max_speed * 0.3)
        } else {
            0.0
        };
        edge_delta = speed * dt;
    }
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    (edge_delta, wheel)
}

/// 应用滚动偏移（确保 scroll_with_delta 被正确的 ScrollArea 消费）
pub(crate) fn apply_scroll_delta(ui: &mut egui::Ui, edge_delta: f32, wheel_delta: f32) {
    if edge_delta != 0.0 {
        ui.scroll_with_delta_animation(
            egui::vec2(0.0, edge_delta),
            egui::style::ScrollAnimation::none(),
        );
        ui.ctx().request_repaint();
    }
    if wheel_delta != 0.0 {
        ui.scroll_with_delta_animation(
            egui::vec2(0.0, wheel_delta),
            egui::style::ScrollAnimation::none(),
        );
        ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
    }
}

/// 构建代码编辑器通用的 TextEdit（无边框、等宽字体、满宽、自定义 layouter）
pub(crate) fn code_text_edit<'t>(
    text: &'t mut dyn egui::TextBuffer,
    id: egui::Id,
    font: egui::FontId,
    layouter: &'t mut dyn FnMut(
        &egui::Ui,
        &dyn egui::TextBuffer,
        f32,
    ) -> std::sync::Arc<egui::Galley>,
) -> egui::TextEdit<'t> {
    egui::TextEdit::multiline(text)
        .id(id)
        .desired_width(f32::INFINITY)
        .font(font)
        .frame(egui::Frame::NONE)
        .layouter(layouter)
}

/// 视窗→普通模式过渡帧 overlay
fn paint_transition_overlay(ui: &egui::Ui, theme: &CodeViewTheme) {
    let clip = ui.clip_rect();
    let painter = ui.painter();
    painter.rect_filled(clip, 0.0, egui::Color32::from_black_alpha(120));
    let font = egui::FontId::proportional(14.0);
    painter.text(
        clip.center(),
        egui::Align2::CENTER_CENTER,
        "Switching...",
        font,
        theme.line_number_color,
    );
}
