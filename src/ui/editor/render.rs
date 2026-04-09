//! 编辑器视图渲染：虚拟滚动代码视图 + Hex 视图
//!
//! 不自建 ScrollArea，复用 egui_dock 外层的 ScrollArea。
//! 通过 set_min_size 撑开内容区域（滚动条由外层计算），
//! 通过 clip_rect 判断可见行，只渲染视口内的行。
//!
//! @author sky

use super::find::FindMatch;
use super::highlight;
use super::tab::{CodeData, EditorTab};
use crate::shell::theme;
use eframe::egui;

/// 行号栏右侧到文本的间距
const GUTTER_PAD: f32 = 8.0;
/// 行内左侧 padding
const TEXT_PAD_LEFT: f32 = 8.0;
/// 代码字体大小
const CODE_FONT_SIZE: f32 = 13.0;

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
    // 分配完整内容区域，让外层 ScrollArea 正确计算滚动范围
    let (content_rect, _) =
        ui.allocate_exact_size(egui::vec2(content_w, total_h), egui::Sense::hover());
    let top_y = content_rect.top() + 4.0;
    // 通过 clip_rect 判断可见行范围
    let clip = ui.clip_rect();
    let first = ((clip.top() - top_y) / line_height).max(0.0) as usize;
    let last = ((clip.bottom() - top_y) / line_height + 1.0).ceil() as usize;
    let last = last.min(line_count);
    let painter = ui.painter();
    // 行号固定在视口左侧，不随水平滚动移动
    let gutter_right_x = clip.left() + gutter_w;
    // 代码内容跟随水平滚动
    let code_x = content_rect.left() + gutter_w + GUTTER_PAD + TEXT_PAD_LEFT;
    // 代码区域裁剪，防止代码滚动到行号栏下方
    let code_clip =
        egui::Rect::from_min_max(egui::pos2(gutter_right_x, clip.top()), clip.right_bottom());
    let code_painter = painter.with_clip_rect(code_clip);
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
    tab: &EditorTab,
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
    );
}

/// Hex 视图
pub fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let theme = super::style::hex::hex_theme();
    egui_hex_view::show(ui, &tab.raw_bytes, &mut tab.hex_state, &theme);
}
