//! 通用代码 TextEdit 构建
//!
//! @author sky

use eframe::egui;
use std::sync::Arc;

/// 构建代码编辑器通用的 TextEdit（无边框、等宽字体、满宽、自定义 layouter）
pub(crate) fn code_text_edit<'t>(
    text: &'t mut dyn egui::TextBuffer,
    id: egui::Id,
    font: egui::FontId,
    layouter: &'t mut dyn FnMut(&egui::Ui, &dyn egui::TextBuffer, f32) -> Arc<egui::Galley>,
) -> egui::TextEdit<'t> {
    egui::TextEdit::multiline(text)
        .id(id)
        .desired_width(f32::INFINITY)
        .font(font)
        .frame(egui::Frame::NONE)
        .layouter(layouter)
}
