//! Codicon 图标 codepoint 常量
//!
//! @author sky

#![allow(dead_code)]

use eframe::egui;

// 窗口控制
pub const CHROME_CLOSE: &str = "\u{EAB8}";
pub const CHROME_MAXIMIZE: &str = "\u{EAB9}";
pub const CHROME_MINIMIZE: &str = "\u{EABA}";
pub const CHROME_RESTORE: &str = "\u{EABB}";
// 文件系统
pub const FILE: &str = "\u{EA7B}";
pub const FOLDER: &str = "\u{EA83}";
pub const FOLDER_OPENED: &str = "\u{EAF7}";
// 通用 UI
pub const SEARCH: &str = "\u{EA6D}";
pub const SETTINGS_GEAR: &str = "\u{EB51}";
pub const CHEVRON_RIGHT: &str = "\u{EAB6}";
pub const CHEVRON_DOWN: &str = "\u{EAB4}";

/// Codicon 字体族
pub fn family() -> egui::FontFamily {
    egui::FontFamily::Name("codicon".into())
}
