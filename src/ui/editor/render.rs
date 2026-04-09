//! 编辑器视图渲染：Decompiled / Bytecode / Hex + 行号绘制
//!
//! @author sky

use super::tab::EditorTab;
use crate::shell::theme;
use eframe::egui;
use egui::TextBuffer;
use std::ops::Range;

/// 只读文本缓冲区，允许选择和复制但禁止编辑
struct ReadOnlyBuffer<'a>(&'a str);

impl TextBuffer for ReadOnlyBuffer<'_> {
    fn is_mutable(&self) -> bool {
        false
    }
    fn as_str(&self) -> &str {
        self.0
    }
    fn insert_text(&mut self, _text: &str, _char_index: usize) -> usize {
        0
    }
    fn delete_char_range(&mut self, _char_range: Range<usize>) {}
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<ReadOnlyBuffer<'static>>()
    }
}

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
    painter.rect_filled(full_rect, 0.0, theme::BG_DARKEST);
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

/// 带行号 + 语法高亮的代码视图通用渲染
fn render_code_view(
    ui: &mut egui::Ui,
    id_salt: &str,
    text: &mut dyn TextBuffer,
    layouter: &mut dyn FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley>,
) {
    let line_count = text.as_str().lines().count().max(1);
    let gutter_w = line_number_width(line_count);
    let min = egui::vec2(ui.available_width(), ui.available_height());
    let response = ui.add(
        egui::TextEdit::multiline(text)
            .id_salt(id_salt)
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: (gutter_w + GUTTER_PAD) as i8,
                right: 8,
                top: 4,
                bottom: 4,
            }))
            .min_size(min)
            .desired_width(f32::INFINITY)
            .layouter(&mut |ui, s, wrap_width| layouter(ui, s.as_str(), wrap_width)),
    );
    paint_line_numbers(ui, response.rect, text.as_str(), gutter_w);
}

/// 反编译视图：只读可选中，带语法高亮
pub fn render_decompiled(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let salt = tab.entry_path.as_deref().unwrap_or(&tab.title);
    let id = format!("dec_{salt}");
    let layouter = &mut tab.layouter_decompiled;
    let mut buf = ReadOnlyBuffer(&tab.decompiled);
    render_code_view(ui, &id, &mut buf, layouter);
}

/// 字节码视图：可编辑，带语法高亮
pub fn render_bytecode(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let salt = tab.entry_path.as_deref().unwrap_or(&tab.title);
    let id = format!("bc_{salt}");
    let layouter = &mut tab.layouter_bytecode;
    render_code_view(ui, &id, &mut tab.bytecode, layouter);
}

/// Hex 视图：自绘 HexGrid（字节级点击 + 双向联动高亮）
pub fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let theme = super::style::hex::hex_theme();
    egui_hex_view::show(ui, &tab.raw_bytes, &mut tab.hex_state, &theme);
}
