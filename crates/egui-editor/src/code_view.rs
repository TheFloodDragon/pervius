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
use eframe::egui;
use std::sync::Arc;

/// 行号栏右侧到文本的间距
const GUTTER_PAD: f32 = 8.0;
/// 行内左侧 padding
const TEXT_PAD_LEFT: f32 = 8.0;

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
            egui::vec2(gutter_w, full_rect.height()),
        ),
        0.0,
        theme.gutter_bg,
    );
}

/// 只读代码视图（TextEdit + 语法高亮 + 搜索高亮 + 行号）
///
/// `line_mapping` 为空时显示顺序行号（1-indexed），
/// 非空时显示原始源码行号，无映射行不显示行号。
pub fn code_view(
    ui: &mut egui::Ui,
    text: &str,
    spans: &[Span],
    line_mapping: &[Option<u32>],
    matches: &[FindMatch],
    current: Option<usize>,
    theme: &CodeViewTheme,
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
    // layouter: 语法高亮 + 搜索匹配背景
    let mut layouter =
        |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| -> Arc<egui::Galley> {
            let s = text_buf.as_str();
            let job = highlight::build_layout_job_with_matches(s, spans, match_ref, current, theme);
            ui.fonts_mut(|f| f.layout_job(job))
        };
    let mut galley_y = 0.0f32;
    let mut edge_scroll_delta = 0.0f32;
    let mut wheel_delta = 0.0f32;
    ui.horizontal_top(|ui| {
        ui.add_space(gutter_w + GUTTER_PAD + TEXT_PAD_LEFT);
        let mut buf: &str = text;
        let output = egui::TextEdit::multiline(&mut buf)
            .desired_width(f32::INFINITY)
            .font(code_font.clone())
            .frame(egui::Frame::NONE)
            .layouter(&mut layouter)
            .show(ui);
        galley_y = output.galley_pos.y;
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
    );
}

/// 可编辑代码视图（TextEdit + 语法高亮 + 搜索高亮 + 行号）
///
/// 返回 `true` 表示文本已被修改，调用方负责刷新高亮数据和标记 tab 状态。
pub fn code_view_editable(
    ui: &mut egui::Ui,
    text: &mut String,
    lang: Language,
    matches: &[FindMatch],
    current: Option<usize>,
    theme: &CodeViewTheme,
) -> bool {
    let line_count = text.split('\n').count().max(1);
    let gutter_w = line_number_width(line_count);
    let code_font = egui::FontId::monospace(theme.code_font_size);
    let match_ranges: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    let match_ref = match_ranges.as_slice();
    // layouter 内实时计算 spans，确保与当前文本同步（消除编辑闪烁）
    let mut layouter =
        |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| -> Arc<egui::Galley> {
            let s = text_buf.as_str();
            let spans = highlight::compute_spans(s, lang);
            let job =
                highlight::build_layout_job_with_matches(s, &spans, match_ref, current, theme);
            ui.fonts_mut(|f| f.layout_job(job))
        };
    let mut galley_y = 0.0f32;
    let mut edge_scroll_delta = 0.0f32;
    let mut wheel_delta = 0.0f32;
    let mut changed = false;
    ui.horizontal_top(|ui| {
        ui.add_space(gutter_w + GUTTER_PAD + TEXT_PAD_LEFT);
        let output = egui::TextEdit::multiline(text)
            .desired_width(f32::INFINITY)
            .font(code_font.clone())
            .frame(egui::Frame::NONE)
            .layouter(&mut layouter)
            .show(ui);
        galley_y = output.galley_pos.y;
        changed = output.response.changed();
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
    );
    changed
}

/// 行号 overlay
fn paint_line_numbers(
    ui: &egui::Ui,
    galley_y: f32,
    line_count: usize,
    line_mapping: &[Option<u32>],
    gutter_w: f32,
    font: &egui::FontId,
    theme: &CodeViewTheme,
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
                egui::pos2(clip.left(), gutter_top),
                egui::pos2(clip.left() + gutter_w + GUTTER_PAD, gutter_bottom),
            ),
            0.0,
            theme.gutter_bg,
        );
    }
    let gutter_right_x = clip.left() + gutter_w;
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
fn detect_edge_scroll(response: &egui::Response, ui: &egui::Ui) -> (f32, f32) {
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
fn apply_scroll_delta(ui: &mut egui::Ui, edge_delta: f32, wheel_delta: f32) {
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
