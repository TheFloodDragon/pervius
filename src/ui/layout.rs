//! 主布局：组装 Sidebar / Explorer / Editor / StatusBar
//!
//! @author sky

use super::editor::ContentArea;
use super::explorer::FilePanel;
use super::sidebar::Sidebar;
use super::status_bar::StatusBar;
use crate::shell::theme;
use eframe::egui;

/// 主布局状态
pub struct Layout {
    pub sidebar: Sidebar,
    pub file_panel: FilePanel,
    pub content_area: ContentArea,
}

impl Layout {
    pub fn new() -> Self {
        use super::demo;
        Self {
            sidebar: Sidebar::default(),
            file_panel: FilePanel::new(demo::tree_nodes(), demo::search_results()),
            content_area: ContentArea::new(demo::tabs(), demo::code_lines()),
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let total = ui.max_rect();
        let mid_top = total.top() + theme::BORDER_WIDTH;
        let mid_bottom = total.bottom() - theme::STATUS_BAR_HEIGHT;
        // 顶部分隔线
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                total.left_top(),
                egui::vec2(total.width(), theme::BORDER_WIDTH),
            ),
            0.0,
            theme::BORDER,
        );
        // 侧边栏
        let sidebar_rect = egui::Rect::from_min_size(
            egui::pos2(total.left(), mid_top),
            egui::vec2(theme::SIDEBAR_WIDTH, mid_bottom - mid_top),
        );
        self.sidebar
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(sidebar_rect)));
        let mut x = sidebar_rect.right();
        x = Self::vsep(ui, x, mid_top, mid_bottom);
        // 文件面板
        let fp_rect = egui::Rect::from_min_size(
            egui::pos2(x, mid_top),
            egui::vec2(theme::FILE_PANEL_WIDTH, mid_bottom - mid_top),
        );
        let active_panel = self.sidebar.active;
        self.file_panel.render(
            &mut ui.new_child(egui::UiBuilder::new().max_rect(fp_rect)),
            active_panel,
        );
        x = fp_rect.right();
        x = Self::vsep(ui, x, mid_top, mid_bottom);
        // 内容区
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(x, mid_top),
            egui::pos2(total.right(), mid_bottom),
        );
        self.content_area
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(content_rect)));
        // 状态栏
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(total.left(), mid_bottom),
            egui::vec2(total.width(), theme::STATUS_BAR_HEIGHT),
        );
        StatusBar::render(&mut ui.new_child(egui::UiBuilder::new().max_rect(status_rect)));
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
