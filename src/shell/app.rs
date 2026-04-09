//! Shell 应用包装器：管理壳子生命周期，业务通过 trait 注入
//!
//! @author sky

use super::{fonts, platform, theme, titlebar};
use eframe::egui;

/// 业务层只需实现这个 trait
pub trait AppContent {
    /// 在 CentralPanel 内绘制业务 UI
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
    /// 标题栏菜单内容（注入到标题栏左侧）
    fn menu_bar(&mut self, _ui: &mut egui::Ui) {}
}

/// 窗口启动配置
pub struct ShellOptions {
    pub title: String,
    pub size: [f32; 2],
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            title: "Pervius".to_owned(),
            size: [1280.0, 800.0],
        }
    }
}

/// 一行启动窗口壳子
pub fn run<F>(options: ShellOptions, create: F) -> eframe::Result
where
    F: FnOnce(&eframe::CreationContext<'_>) -> Box<dyn AppContent> + 'static,
{
    let viewport = platform::viewport(&options.title, options.size);
    let native = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let title = options.title.clone();
    eframe::run_native(
        &options.title,
        native,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals({
                let mut v = egui::Visuals::dark();
                // #1A1A21 — 编辑器代码区背景，替换 egui 默认的 rgb(10,10,10)
                v.extreme_bg_color = theme::BG_MEDIUM;
                v.code_bg_color = theme::BG_MEDIUM;
                v
            });
            fonts::setup(&cc.egui_ctx);
            let content = create(cc);
            Ok(Box::new(ShellApp {
                title,
                content,
                #[cfg(target_os = "windows")]
                corners_set: false,
            }))
        }),
    )
}

struct ShellApp {
    title: String,
    content: Box<dyn AppContent>,
    #[cfg(target_os = "windows")]
    corners_set: bool,
}

impl eframe::App for ShellApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(target_os = "windows")]
        if !self.corners_set {
            platform::enable_rounded_corners(&self.title);
            self.corners_set = true;
        }
        // 保存完整窗口 rect（titlebar Panel 会缩小 max_rect）
        #[cfg(not(target_os = "macos"))]
        let full_rect = ui.max_rect();
        titlebar::render(ui, &self.title, |ui| self.content.menu_bar(ui));
        // 窗口边缘 resize 在 titlebar 之后注册（后注册优先级更高，覆盖 titlebar drag）
        #[cfg(not(target_os = "macos"))]
        handle_window_resize(ui, &ctx, full_rect);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG_DARK))
            .show_inside(ui, |ui| {
                self.content.ui(ui, &ctx);
            });
    }
}

/// 在窗口边缘检测拖拽并触发系统 resize
#[cfg(not(target_os = "macos"))]
fn handle_window_resize(ui: &mut egui::Ui, ctx: &egui::Context, rect: egui::Rect) {
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if maximized {
        return;
    }
    let margin = 5.0;
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let pos = match pointer {
        Some(p) if rect.contains(p) => p,
        _ => return,
    };
    let left = pos.x - rect.left() < margin;
    let right = rect.right() - pos.x < margin;
    let top = pos.y - rect.top() < margin;
    let bottom = rect.bottom() - pos.y < margin;
    let dir = match (left, right, top, bottom) {
        (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
        (_, true, true, _) => Some(egui::ResizeDirection::NorthEast),
        (true, _, _, true) => Some(egui::ResizeDirection::SouthWest),
        (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
        (true, _, _, _) => Some(egui::ResizeDirection::West),
        (_, true, _, _) => Some(egui::ResizeDirection::East),
        (_, _, true, _) => Some(egui::ResizeDirection::North),
        (_, _, _, true) => Some(egui::ResizeDirection::South),
        _ => None,
    };
    if let Some(d) = dir {
        let cursor = match d {
            egui::ResizeDirection::North | egui::ResizeDirection::South => {
                egui::CursorIcon::ResizeVertical
            }
            egui::ResizeDirection::West | egui::ResizeDirection::East => {
                egui::CursorIcon::ResizeHorizontal
            }
            egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                egui::CursorIcon::ResizeNwSe
            }
            egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                egui::CursorIcon::ResizeNeSw
            }
        };
        ctx.set_cursor_icon(cursor);
        // 用完整窗口 rect 注册 interact，后注册覆盖 titlebar 的 drag
        let edge_id = ui.id().with("window_resize");
        let response = ui.interact(rect, edge_id, egui::Sense::drag());
        if response.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
        }
    }
}
