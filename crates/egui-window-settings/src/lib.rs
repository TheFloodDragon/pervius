//! 设置面板框架：FloatingWindow 承载的侧栏 + 内容分栏布局 + 通用 widget 原语
//!
//! 颜色通过 [`SettingsTheme`] 传入，不硬编码任何色值。
//!
//! @author sky

mod panel;
mod persist;
mod widget;

use eframe::egui;

pub use panel::SettingsPanel;
pub use persist::SettingsFile;
pub use widget::{dropdown, path_picker, section_header, sidebar_item, slider, toggle};

/// 设置面板主题配色
pub struct SettingsTheme {
    /// 主要文字色
    pub text_primary: egui::Color32,
    /// 次要文字色
    pub text_secondary: egui::Color32,
    /// 暗淡文字色
    pub text_muted: egui::Color32,
    /// 悬停背景
    pub bg_hover: egui::Color32,
    /// 浅层背景（toggle off、slider 轨道等）
    pub bg_light: egui::Color32,
    /// 中层背景（按钮底色等）
    pub bg_medium: egui::Color32,
    /// 侧栏背景
    pub bg_sidebar: egui::Color32,
    /// 边框 / 分隔线
    pub border: egui::Color32,
    /// 强调色（toggle on、slider 填充、active 项等）
    pub accent: egui::Color32,
    /// 图标字体族
    pub icon_font: egui::FontFamily,
    /// 下拉箭头字符
    pub chevron_icon: char,
}
