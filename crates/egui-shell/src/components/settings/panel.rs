//! 设置面板：FloatingWindow + 侧栏/内容分栏布局
//!
//! @author sky

use super::SettingsTheme;
use crate::components::window::FloatingWindow;
use crate::components::WindowTheme;
use eframe::egui;

/// 侧栏宽度
const SIDEBAR_W: f32 = 130.0;

/// 设置面板：FloatingWindow + 侧栏/内容分栏布局
pub struct SettingsPanel {
    window: FloatingWindow,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            window: FloatingWindow::default(),
        }
    }
}

impl SettingsPanel {
    pub fn new(id: impl Into<egui::Id>, title: impl Into<String>) -> Self {
        Self {
            window: FloatingWindow::new(id, title),
        }
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.window = self.window.icon(icon);
        self
    }

    pub fn default_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.default_size(size);
        self
    }

    pub fn min_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.window = self.window.min_size(size);
        self
    }

    pub fn open(&mut self) {
        self.window.open();
    }

    pub fn close(&mut self) {
        self.window.close();
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    /// 渲染面板，回调接收 `(sidebar_ui, content_ui)` 两个独立的绘制区域
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        wt: &WindowTheme,
        theme: &SettingsTheme,
        body: impl FnOnce(&mut egui::Ui, &mut egui::Ui),
    ) {
        self.window.show(
            ctx,
            wt,
            |_ui| {},
            |ui| {
                let total = ui.available_rect_before_wrap();
                let sw = SIDEBAR_W.min(total.width() * 0.5);
                // 侧栏背景
                let sidebar_rect =
                    egui::Rect::from_min_size(total.left_top(), egui::vec2(sw, total.height()));
                ui.painter()
                    .rect_filled(sidebar_rect, 0.0, theme.bg_sidebar);
                // 侧栏 Ui
                let mut sidebar_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id(egui::Id::new("settings_sidebar"))
                        .max_rect(sidebar_rect),
                );
                sidebar_ui.set_clip_rect(sidebar_rect);
                // 垂直分隔线
                ui.painter().line_segment(
                    [
                        egui::pos2(sidebar_rect.right(), total.top()),
                        egui::pos2(sidebar_rect.right(), total.bottom()),
                    ],
                    egui::Stroke::new(1.0, theme.border),
                );
                // 右侧内容区
                let content_left = (sidebar_rect.right() + 1.0).min(total.right());
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(content_left, total.top()),
                    total.right_bottom(),
                );
                if content_rect.width() > 1.0 {
                    let mut content_outer = ui.new_child(
                        egui::UiBuilder::new()
                            .id(egui::Id::new("settings_content"))
                            .max_rect(content_rect),
                    );
                    content_outer.set_clip_rect(content_rect);
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .show(&mut content_outer, |content_ui| {
                            body(&mut sidebar_ui, content_ui);
                        });
                }
                ui.allocate_rect(total, egui::Sense::hover());
            },
        );
    }
}
