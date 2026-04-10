//! 可复用 UI 组件：浮动窗口、设置面板
//!
//! @author sky

pub mod settings;
pub mod window;

use eframe::egui;

pub use settings::{
    dropdown, is_recording_keybind, keybind_row, keybind_row_with, path_picker, path_picker_with,
    section_header, sidebar_item, slider, toggle, SettingsFile, SettingsPanel, SettingsTheme,
};
pub use window::FloatingWindow;

/// 浮动窗口主题配色
#[derive(Clone)]
pub struct WindowTheme {
    /// 窗口外框样式（fill / stroke / corner_radius / shadow）
    pub frame: egui::Frame,
    /// header 区域高度
    pub header_height: f32,
    /// 强调色（图标、active pin）
    pub accent: egui::Color32,
    /// 主文字色（标题）
    pub text_primary: egui::Color32,
    /// 暗淡文字色（inactive pin）
    pub text_muted: egui::Color32,
    /// 按钮 active 底色（pin 激活时）
    pub bg_active: egui::Color32,
    /// widget hover 底色
    pub bg_hover: egui::Color32,
    /// widget pressed 底色
    pub bg_pressed: egui::Color32,
    /// 分隔线颜色
    pub separator: egui::Color32,
    /// 图标字体族（header icon + pin icon）
    pub icon_font: egui::FontFamily,
    /// Pin 图标字符
    pub pin_icon: &'static str,
    /// Pin 按钮 tooltip（已固定时）
    pub pin_tooltip: String,
    /// Unpin 按钮 tooltip（未固定时）
    pub unpin_tooltip: String,
}

/// 1px 水平分隔线
pub fn separator(ui: &mut egui::Ui, color: egui::Color32) {
    let avail = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(avail.left(), avail.top()),
            egui::pos2(avail.right(), avail.top()),
        ],
        egui::Stroke::new(1.0, color),
    );
    ui.allocate_space(egui::vec2(avail.width(), 1.0));
}
