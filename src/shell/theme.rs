//! 主题色常量
//!
//! 完整色板对照 palette.slint，所有 hex 值标注在注释中。
//!
//! @author sky

#![allow(dead_code)]

use eframe::egui;

// -- 背景层级 --

/// 最深背景 #0C0C0F（HeaderBar、Sidebar、StatusBar）
pub const BG_DARKEST: egui::Color32 = egui::Color32::from_rgb(12, 12, 15);
/// 主背景 #131318（面板底色）
pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(19, 19, 24);
/// 中层背景 #1A1A21（输入框、编辑区、ViewToggle 容器）
pub const BG_MEDIUM: egui::Color32 = egui::Color32::from_rgb(26, 26, 33);
/// 浅层背景 #23232D（关闭按钮 hover 等）
pub const BG_LIGHT: egui::Color32 = egui::Color32::from_rgb(35, 35, 45);
/// 悬停背景 #2A2A36
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 54);

// -- 边框 --

/// 主边框 #2A2A35
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(42, 42, 53);
/// 浅边框 #363645
pub const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgb(54, 54, 69);

// -- 文字 --

/// 主要文字 #ECECEF
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(236, 236, 239);
/// 次要文字 #A0A0AB
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(160, 160, 171);
/// 暗淡文字 #5C5C6A
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(92, 92, 106);

// -- 强调色 --

/// 主色调铜绿 #43B3AE
pub const VERDIGRIS: egui::Color32 = egui::Color32::from_rgb(67, 179, 174);
/// 辅助绿 #6EE7B7
pub const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(110, 231, 183);
/// 辅助橙 #E0A458
pub const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(224, 164, 88);
/// 辅助红 #E06C75
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(224, 108, 117);
/// 辅助青 #7EC8C8
pub const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(126, 200, 200);

// -- 语法高亮（JetBrains Darcula）--

/// 关键字 #CC7832
pub const SYN_KEYWORD: egui::Color32 = egui::Color32::from_rgb(204, 120, 50);
/// 字符串 #7DA668（原 #6A8759 偏亮）
pub const SYN_STRING: egui::Color32 = egui::Color32::from_rgb(125, 166, 104);
/// 类型 #A9B7C6
pub const SYN_TYPE: egui::Color32 = egui::Color32::from_rgb(169, 183, 198);
/// 数字 #6897BB
pub const SYN_NUMBER: egui::Color32 = egui::Color32::from_rgb(104, 151, 187);
/// 注释 #808080
pub const SYN_COMMENT: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
/// 注解 #BBB529
pub const SYN_ANNOTATION: egui::Color32 = egui::Color32::from_rgb(187, 181, 41);
/// 方法调用 #FFC66D
pub const SYN_METHOD: egui::Color32 = egui::Color32::from_rgb(255, 198, 109);
/// 方法声明 #51A7E4
pub const SYN_METHOD_DECL: egui::Color32 = egui::Color32::from_rgb(81, 167, 228);

// -- 标题栏控制按钮 --

/// 标题栏按钮 hover #2A2A2F
pub const CAPTION_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 47);
/// 关闭按钮 hover #C42B1C（Win11 风格红）
pub const CLOSE_HOVER: egui::Color32 = egui::Color32::from_rgb(196, 43, 28);

// -- 尺寸 --

/// 标题栏高度
pub const TITLE_BAR_HEIGHT: f32 = 36.0;
/// 侧边栏宽度
pub const SIDEBAR_WIDTH: f32 = 48.0;
/// 文件面板宽度
pub const FILE_PANEL_WIDTH: f32 = 260.0;
/// Tab 栏高度
pub const TAB_BAR_HEIGHT: f32 = 34.0;
/// 状态栏高度
pub const STATUS_BAR_HEIGHT: f32 = 24.0;
/// 视图切换工具栏高度
pub const VIEW_TOOLBAR_HEIGHT: f32 = 36.0;
/// 边框宽度
pub const BORDER_WIDTH: f32 = 1.0;

/// 铜绿色带透明度（用于选中高亮等）
pub fn verdigris_alpha(alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(
        (67u16 * alpha as u16 / 255) as u8,
        (179u16 * alpha as u16 / 255) as u8,
        (174u16 * alpha as u16 / 255) as u8,
        alpha,
    )
}
