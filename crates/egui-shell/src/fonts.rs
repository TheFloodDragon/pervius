//! 字体加载
//!
//! @author sky

use super::codicon;
use eframe::egui;

/// 注册 Codicon 字体到 egui 字体系统
pub fn setup(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/codicon.ttf")).into(),
    );
    fonts
        .families
        .entry(codicon::family())
        .or_default()
        .push("codicon".to_owned());
    ctx.set_fonts(fonts);
}
