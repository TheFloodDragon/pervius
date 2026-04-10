//! 编辑器视图渲染：虚拟滚动代码视图 + Hex 视图
//!
//! 不自建 ScrollArea，复用 egui_dock 外层的 ScrollArea。
//! 通过 set_min_size 撑开内容区域（滚动条由外层计算），
//! 通过 clip_rect 判断可见行，只渲染视口内的行。
//!
//! @author sky

use super::find::FindMatch;
use super::highlight;
use super::tab::{CodeData, EditorTab, TextPosition, TextSelection};
use crate::shell::theme;
use eframe::egui;

/// 行号栏右侧到文本的间距
const GUTTER_PAD: f32 = 8.0;
/// 行内左侧 padding
const TEXT_PAD_LEFT: f32 = 8.0;
/// 代码字体大小
const CODE_FONT_SIZE: f32 = 13.0;
/// 选中区域背景色透明度
const SELECTION_ALPHA: u8 = 45;

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
pub fn paint_editor_bg(ui: &egui::Ui, full_rect: egui::Rect, gutter_w: f32) {
    let painter = ui.painter();
    painter.rect_filled(full_rect, 0.0, theme::BG_DARKEST);
    painter.rect_filled(
        egui::Rect::from_min_size(
            full_rect.left_top(),
            egui::vec2(gutter_w, full_rect.height()),
        ),
        0.0,
        theme::BG_GUTTER,
    );
}

// -- 选择辅助 --

/// 屏幕坐标 → 文本位置（行 + 列）
fn hit_test(
    pos: egui::Pos2,
    top_y: f32,
    code_x: f32,
    line_height: f32,
    char_w: f32,
    text: &str,
    data: &CodeData,
) -> TextPosition {
    let line = ((pos.y - top_y) / line_height).floor().max(0.0) as usize;
    let line = line.min(data.line_count().saturating_sub(1));
    let col = ((pos.x - code_x) / char_w).round().max(0.0) as usize;
    let line_text = highlight::get_line(text, &data.line_starts, line);
    let max_col = line_text.chars().count();
    TextPosition {
        line,
        col: col.min(max_col),
    }
}

/// 在行内查找光标所在 word 的边界（字符索引）
fn find_word_bounds(line: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() {
        return (chars.len(), chars.len());
    }
    let ch = chars[col];
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    if !is_word(ch) {
        return (col, col + 1);
    }
    let mut start = col;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    (start, end)
}

/// TextPosition → 文本字节偏移
fn pos_to_byte(text: &str, data: &CodeData, pos: TextPosition) -> usize {
    if pos.line >= data.line_count() {
        return text.len();
    }
    let line_start = data.line_starts[pos.line];
    let line_text = highlight::get_line(text, &data.line_starts, pos.line);
    let byte_off: usize = line_text.chars().take(pos.col).map(|c| c.len_utf8()).sum();
    (line_start + byte_off).min(text.len())
}

/// 从选区中提取文本
fn extract_selected_text(text: &str, data: &CodeData, sel: &TextSelection) -> String {
    if !sel.active {
        return String::new();
    }
    let (start, end) = ordered_range(sel);
    let sb = pos_to_byte(text, data, start);
    let eb = pos_to_byte(text, data, end);
    if sb >= eb || sb >= text.len() {
        return String::new();
    }
    text[sb..eb.min(text.len())].to_string()
}

/// 返回选区的有序 (start, end)
fn ordered_range(sel: &TextSelection) -> (TextPosition, TextPosition) {
    if sel.anchor <= sel.cursor {
        (sel.anchor, sel.cursor)
    } else {
        (sel.cursor, sel.anchor)
    }
}

/// 处理选择交互（click / double-click / shift+click / drag）
fn handle_selection(
    response: &egui::Response,
    ui: &egui::Ui,
    selection: &mut TextSelection,
    text: &str,
    data: &CodeData,
    top_y: f32,
    code_x: f32,
    line_height: f32,
    char_w: f32,
) {
    let shift = ui.input(|i| i.modifiers.shift);
    if response.double_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tp = hit_test(pos, top_y, code_x, line_height, char_w, text, data);
            let line_text = highlight::get_line(text, &data.line_starts, tp.line);
            let (ws, we) = find_word_bounds(line_text, tp.col);
            selection.anchor = TextPosition {
                line: tp.line,
                col: ws,
            };
            selection.cursor = TextPosition {
                line: tp.line,
                col: we,
            };
            selection.active = ws != we;
        }
        return;
    }
    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tp = hit_test(pos, top_y, code_x, line_height, char_w, text, data);
            if shift && selection.active {
                selection.cursor = tp;
            } else {
                selection.anchor = tp;
                selection.cursor = tp;
            }
            selection.active = selection.anchor != selection.cursor;
        }
    }
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tp = hit_test(pos, top_y, code_x, line_height, char_w, text, data);
            selection.cursor = tp;
            selection.active = selection.anchor != selection.cursor;
        }
    } else if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tp = hit_test(pos, top_y, code_x, line_height, char_w, text, data);
            if shift && selection.active {
                selection.cursor = tp;
            } else {
                selection.anchor = tp;
                selection.cursor = tp;
            }
            selection.active = selection.anchor != selection.cursor;
        }
    }
}

/// 处理键盘快捷键（Ctrl+C / Ctrl+A）
fn handle_keyboard(
    ui: &egui::Ui,
    response: &egui::Response,
    selection: &mut TextSelection,
    text: &str,
    data: &CodeData,
) {
    let focused = response.has_focus();
    // Ctrl+C
    if selection.active && focused {
        let copy = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C));
        if copy {
            let selected = extract_selected_text(text, data, selection);
            if !selected.is_empty() {
                ui.ctx().copy_text(selected);
            }
        }
    }
    // Ctrl+A
    if focused {
        let select_all = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A));
        if select_all && data.line_count() > 0 {
            let last_line = data.line_count() - 1;
            let last_col = highlight::get_line(text, &data.line_starts, last_line)
                .chars()
                .count();
            selection.anchor = TextPosition::default();
            selection.cursor = TextPosition {
                line: last_line,
                col: last_col,
            };
            selection.active = true;
        }
    }
}

/// 虚拟滚动代码视图（不自建 ScrollArea，借用外层 egui_dock 的）
///
/// `line_mapping` 为空时显示顺序行号（1-indexed），
/// 非空时显示原始源码行号，无映射行不显示行号。
fn render_code_view(
    ui: &mut egui::Ui,
    text: &str,
    data: &CodeData,
    line_mapping: &[Option<u32>],
    matches: &[FindMatch],
    current: Option<usize>,
    selection: &mut TextSelection,
) {
    let line_count = data.line_count();
    // gutter 宽度按最大行号计算
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
    let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let gutter_font = egui::FontId::monospace(CODE_FONT_SIZE);
    // monospace 字符宽度估算
    let char_w = line_height * 0.6;
    let content_w =
        gutter_w + GUTTER_PAD + TEXT_PAD_LEFT + data.max_line_len as f32 * char_w + 32.0;
    let total_h = line_count as f32 * line_height + 8.0;
    // 分配完整内容区域，支持点击和拖拽选择
    let (content_rect, response) = ui.allocate_exact_size(
        egui::vec2(content_w, total_h),
        egui::Sense::click_and_drag(),
    );
    let top_y = content_rect.top() + 4.0;
    let code_x = content_rect.left() + gutter_w + GUTTER_PAD + TEXT_PAD_LEFT;
    // 选择交互（click / drag / Ctrl+C / Ctrl+A）
    handle_selection(
        &response,
        ui,
        selection,
        text,
        data,
        top_y,
        code_x,
        line_height,
        char_w,
    );
    // 点击时获取焦点，用于键盘快捷键
    if response.clicked() || response.drag_started() {
        response.request_focus();
    }
    handle_keyboard(ui, &response, selection, text, data);
    // 代码区域 hover 时显示文本光标
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    // 通过 clip_rect 判断可见行范围
    let clip = ui.clip_rect();
    let first = ((clip.top() - top_y) / line_height).max(0.0) as usize;
    let last = ((clip.bottom() - top_y) / line_height + 1.0).ceil() as usize;
    let last = last.min(line_count);
    let painter = ui.painter();
    // 行号固定在视口左侧，不随水平滚动移动
    let gutter_right_x = clip.left() + gutter_w;
    // 代码区域裁剪，防止代码滚动到行号栏下方
    let code_clip =
        egui::Rect::from_min_max(egui::pos2(gutter_right_x, clip.top()), clip.right_bottom());
    let code_painter = painter.with_clip_rect(code_clip);
    // 选区有序范围
    let sel_range = if selection.active {
        Some(ordered_range(selection))
    } else {
        None
    };
    for i in first..last {
        let y = top_y + i as f32 * line_height;
        // 行号（固定位置）
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
                gutter_font.clone(),
                theme::TEXT_MUTED,
            );
        }
        // 语法高亮行内容
        let line_text = highlight::get_line(text, &data.line_starts, i);
        let job = highlight::build_single_line_job(line_text, &data.spans, data.line_starts[i]);
        let galley = painter.layout_job(job);
        let line_pos = egui::pos2(code_x, y);
        // 选中高亮（在文本和 find 高亮之前绘制）
        if let Some((start, end)) = sel_range {
            if i >= start.line && i <= end.line {
                let line_chars = line_text.chars().count();
                let sc = if i == start.line { start.col } else { 0 };
                let ec = if i == end.line { end.col } else { line_chars };
                if sc < ec {
                    let r0 = galley.pos_from_cursor(egui::text::CCursor::new(sc));
                    let r1 = galley.pos_from_cursor(egui::text::CCursor::new(ec));
                    let sel_rect = egui::Rect::from_min_max(
                        egui::pos2(line_pos.x + r0.min.x, line_pos.y),
                        egui::pos2(line_pos.x + r1.min.x, line_pos.y + line_height),
                    );
                    code_painter.rect_filled(
                        sel_rect,
                        0.0,
                        theme::verdigris_alpha(SELECTION_ALPHA),
                    );
                } else if i > start.line && i < end.line {
                    // 中间空行也高亮
                    let sel_rect =
                        egui::Rect::from_min_size(line_pos, egui::vec2(char_w * 2.0, line_height));
                    code_painter.rect_filled(
                        sel_rect,
                        0.0,
                        theme::verdigris_alpha(SELECTION_ALPHA),
                    );
                }
            }
        }
        // 绘制该行上的 find 匹配高亮
        if !matches.is_empty() {
            let line_byte_start = data.line_starts[i];
            let line_byte_end = line_byte_start + line_text.len();
            for (mi, m) in matches.iter().enumerate() {
                if m.end <= line_byte_start || m.start >= line_byte_end {
                    continue;
                }
                let ms = m.start.max(line_byte_start) - line_byte_start;
                let me = m.end.min(line_byte_end) - line_byte_start;
                let cs = line_text[..ms].chars().count();
                let ce = line_text[..me].chars().count();
                let r0 = galley.pos_from_cursor(egui::text::CCursor::new(cs));
                let r1 = galley.pos_from_cursor(egui::text::CCursor::new(ce));
                let highlight_rect = egui::Rect::from_min_max(
                    egui::pos2(line_pos.x + r0.min.x, line_pos.y + r0.min.y),
                    egui::pos2(line_pos.x + r1.min.x, line_pos.y + r1.max.y),
                );
                let is_current = current == Some(mi);
                if is_current {
                    code_painter.rect_filled(highlight_rect, 2.0, theme::verdigris_alpha(60));
                    code_painter.rect_stroke(
                        highlight_rect,
                        2.0,
                        egui::Stroke::new(1.0, theme::VERDIGRIS),
                        egui::StrokeKind::Outside,
                    );
                } else {
                    code_painter.rect_filled(highlight_rect, 2.0, theme::verdigris_alpha(25));
                }
            }
        }
        code_painter.galley(line_pos, galley, egui::Color32::PLACEHOLDER);
    }
}

/// 反编译视图
pub fn render_decompiled(
    ui: &mut egui::Ui,
    tab: &mut EditorTab,
    matches: &[FindMatch],
    current: Option<usize>,
) {
    render_code_view(
        ui,
        &tab.decompiled,
        &tab.decompiled_data,
        &tab.decompiled_line_mapping,
        matches,
        current,
        &mut tab.decompiled_selection,
    );
}

/// Hex 视图
pub fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let theme = super::style::hex::hex_theme();
    egui_hex_view::show(ui, &tab.raw_bytes, &mut tab.hex_state, &theme);
}
