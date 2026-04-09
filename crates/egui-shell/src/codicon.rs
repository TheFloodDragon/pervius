//! Codicon 字体：caption button 图标 + 字体族注册
//!
//! @author sky

use eframe::egui;

/// 关闭按钮 U+EAB8
pub const CHROME_CLOSE: &str = "\u{EAB8}";
/// 最大化按钮 U+EAB9
pub const CHROME_MAXIMIZE: &str = "\u{EAB9}";
/// 最小化按钮 U+EABA
pub const CHROME_MINIMIZE: &str = "\u{EABA}";
/// 还原按钮 U+EABB
pub const CHROME_RESTORE: &str = "\u{EABB}";

/// Codicon 字体族名称
pub fn family() -> egui::FontFamily {
    egui::FontFamily::Name("codicon".into())
}
