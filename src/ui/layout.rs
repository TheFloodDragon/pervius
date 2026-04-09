//! 主布局：Explorer / Editor / StatusBar
//!
//! @author sky

use super::editor::EditorArea;
use super::explorer::FilePanel;
use super::status_bar::StatusBar;
use crate::shell::theme;
use eframe::egui;
use egui_notify::Toasts;

/// 主布局状态
pub struct Layout {
    pub file_panel: FilePanel,
    pub editor: EditorArea,
    pub toasts: Toasts,
}

impl Layout {
    pub fn new() -> Self {
        use super::demo;
        Self {
            file_panel: FilePanel::new(demo::tree_nodes(), demo::search_results()),
            editor: EditorArea::new(demo::editor_tabs()),
            toasts: Toasts::default(),
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let total = ui.max_rect();
        let mid_top = total.top();
        let mid_bottom = total.bottom() - theme::STATUS_BAR_HEIGHT;
        // 文件面板
        let mut x = total.left();
        let fp_rect = egui::Rect::from_min_size(
            egui::pos2(x, mid_top),
            egui::vec2(theme::FILE_PANEL_WIDTH, mid_bottom - mid_top),
        );
        self.file_panel
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(fp_rect)));
        x = fp_rect.right();
        x = Self::vsep(ui, x, mid_top, mid_bottom);
        // 内容区（egui_dock 管理）
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(x, mid_top),
            egui::pos2(total.right(), mid_bottom),
        );
        self.editor
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(content_rect)));
        // 状态栏
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(total.left(), mid_bottom),
            egui::vec2(total.width(), theme::STATUS_BAR_HEIGHT),
        );
        let active_view = self.editor.focused_view();
        let new_view = StatusBar::render(
            &mut ui.new_child(egui::UiBuilder::new().max_rect(status_rect)),
            active_view,
        );
        if let Some(v) = new_view {
            self.editor.set_focused_view(v);
        }
        // Toast 通知
        self.toasts.show(ui.ctx());
    }

    fn vsep(ui: &egui::Ui, x: f32, top: f32, bottom: f32) -> f32 {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x, top),
                egui::vec2(theme::BORDER_WIDTH, bottom - top),
            ),
            0.0,
            theme::BORDER,
        );
        x + theme::BORDER_WIDTH
    }
}
