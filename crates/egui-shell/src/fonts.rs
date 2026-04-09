//! 字体加载
//!
//! @author sky

use super::codicon;
use eframe::egui;

/// 内嵌 CJK 字体（Noto Sans SC Regular, OFL 许可）
static CJK_FONT: &[u8] = include_bytes!("../fonts/NotoSansSC-Regular.otf");

/// 注册字体到 egui 字体系统（Codicon 图标 + CJK 回退）
pub fn setup(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Codicon 图标字体
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/codicon.ttf")).into(),
    );
    fonts
        .families
        .entry(codicon::family())
        .or_default()
        .push("codicon".to_owned());
    // CJK 字体作为 Proportional / Monospace 回退
    fonts.font_data.insert(
        "cjk".to_owned(),
        egui::FontData::from_static(CJK_FONT).into(),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}
