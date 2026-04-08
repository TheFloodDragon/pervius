//! Shell 应用包装器：管理壳子生命周期，业务通过 trait 注入
//!
//! @author sky

use super::{fonts, platform, theme, titlebar};
use eframe::egui;

/// 业务层只需实现这个 trait
pub trait AppContent {
    /// 在 CentralPanel 内绘制业务 UI
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context);
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
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
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
        titlebar::render(ui, &self.title);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::BG_DARK))
            .show_inside(ui, |ui| {
                self.content.ui(ui, &ctx);
            });
    }
}
