//! 编辑器视图渲染：Decompiled / Bytecode / Hex + 行号绘制
//!
//! @author sky

use super::tab::EditorTab;
use crate::shell::theme;
use eframe::egui;

/// 行号栏右侧到文本的间距
const GUTTER_PAD: f32 = 8.0;

/// 行号栏宽度（根据行数计算位数）
pub fn line_number_width(line_count: usize) -> f32 {
    let digits = if line_count == 0 {
        1
    } else {
        (line_count as f32).log10().floor() as usize + 1
    };
    // 每位约 8px（monospace 13），加左右各 12px padding
    digits as f32 * 8.0 + 24.0
}

/// 在 ScrollArea 外绘制全高背景（左侧 gutter + 右侧编辑区）
pub fn paint_editor_bg(ui: &egui::Ui, full_rect: egui::Rect, gutter_w: f32) {
    let painter = ui.painter();
    // 右侧编辑区背景
    painter.rect_filled(full_rect, 0.0, theme::BG_MEDIUM);
    // 左侧 gutter 背景覆盖编辑区
    painter.rect_filled(
        egui::Rect::from_min_size(
            full_rect.left_top(),
            egui::vec2(gutter_w, full_rect.height()),
        ),
        0.0,
        theme::BG_GUTTER,
    );
}

/// 在 TextEdit 左侧绘制行号（只画数字，背景由调用方在 ScrollArea 外预先绘制）
fn paint_line_numbers(ui: &egui::Ui, text_rect: egui::Rect, text: &str, gutter_w: f32) {
    let painter = ui.painter();
    let font = egui::FontId::monospace(13.0);
    let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let gutter_right = text_rect.left() + gutter_w;
    let line_count = text.lines().count().max(1);
    let top_y = text_rect.top() + 4.0;
    for i in 0..line_count {
        let y = top_y + i as f32 * line_height;
        if y > text_rect.bottom() {
            break;
        }
        painter.text(
            egui::pos2(gutter_right - 8.0, y),
            egui::Align2::RIGHT_TOP,
            format!("{}", i + 1),
            font.clone(),
            theme::TEXT_MUTED,
        );
    }
}

/// 反编译视图：只读，带语法高亮
pub fn render_decompiled(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let layouter = &mut tab.layouter_decompiled;
    let line_count = tab.decompiled.lines().count().max(1);
    let gutter_w = line_number_width(line_count);
    let min = egui::vec2(ui.available_width(), ui.available_height());
    let response = ui.add(
        egui::TextEdit::multiline(&mut tab.decompiled)
            .id_salt(format!("te_dec_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: (gutter_w + GUTTER_PAD) as i8,
                right: 8,
                top: 4,
                bottom: 4,
            }))
            .interactive(false)
            .min_size(min)
            .desired_width(f32::INFINITY)
            .layouter(&mut |ui, text, wrap_width| layouter(ui, text.as_str(), wrap_width)),
    );
    paint_line_numbers(ui, response.rect, &tab.decompiled, gutter_w);
}

/// 字节码视图：可编辑纯文本
pub fn render_bytecode(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let line_count = tab.bytecode.lines().count().max(1);
    let gutter_w = line_number_width(line_count);
    let min = egui::vec2(ui.available_width(), ui.available_height());
    let response = ui.add(
        egui::TextEdit::multiline(&mut tab.bytecode)
            .id_salt(format!("te_bc_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: (gutter_w + GUTTER_PAD) as i8,
                right: 8,
                top: 4,
                bottom: 4,
            }))
            .min_size(min)
            .desired_width(f32::INFINITY),
    );
    paint_line_numbers(ui, response.rect, &tab.bytecode, gutter_w);
}

/// Hex 视图：只读 hex dump（无行号）
pub fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let min = egui::vec2(ui.available_width(), ui.available_height());
    ui.add(
        egui::TextEdit::multiline(&mut tab.hex_dump)
            .id_salt(format!("te_hex_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_MEDIUM)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .interactive(false)
            .min_size(min)
            .desired_width(f32::INFINITY),
    );
}
