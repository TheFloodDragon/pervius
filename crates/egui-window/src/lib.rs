//! 主题化浮动窗口：Area + Frame 自绘，内置 pin/unpin 逻辑
//!
//! 不使用 egui::Window（其边缘 resize 存在帧同步缺陷），
//! 改为 egui::Area + egui::Frame 手动控制窗口尺寸和位置。
//! Area 设为 movable(false)，移动和 resize 完全由本层代码控制，互不冲突。
//!
//! @author sky

mod window;

use eframe::egui;

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
