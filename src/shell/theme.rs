//! 主题色常量
//!
//! @author sky

use eframe::egui;

/// 全局背景色 #131318
pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(19, 19, 24);
/// 标题栏高度（像素）
pub const TITLE_BAR_HEIGHT: f32 = 36.0;
/// 主色调铜绿 #43B3AE
pub const VERDIGRIS: egui::Color32 = egui::Color32::from_rgb(67, 179, 174);
/// 主要文字色 #ECECEF，近白
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(236, 236, 239);
/// 次要文字色 #999999，标题栏图标默认色
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(153, 153, 153);
/// 标题栏按钮 hover 背景 #2A2A2F
pub const CAPTION_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 47);
/// 关闭按钮 hover 背景 #C42B1C，Win11 风格红
pub const CLOSE_HOVER: egui::Color32 = egui::Color32::from_rgb(196, 43, 28);
