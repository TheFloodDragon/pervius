//! 左侧面板：根据 Sidebar 激活项分发到 tree / search / structure
//!
//! @author sky

pub mod search;
pub mod structure;
pub mod tree;

use crate::shell::theme;
use crate::ui::sidebar::SidebarPanel;
use eframe::egui;
use search::SearchResult;
use tree::TreeNode;

/// 文件面板状态
pub struct FilePanel {
    pub tree_nodes: Vec<TreeNode>,
    pub search_results: Vec<SearchResult>,
    pub selected_index: usize,
}

impl FilePanel {
    pub fn new(tree_nodes: Vec<TreeNode>, search_results: Vec<SearchResult>) -> Self {
        Self {
            tree_nodes,
            search_results,
            selected_index: 0,
        }
    }

    /// 在给定 rect 内渲染
    pub fn render(&mut self, ui: &mut egui::Ui, active: SidebarPanel) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::BG_DARK);
        // 面板标题
        let title_h = 32.0;
        let title_rect =
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(rect.width(), title_h));
        let label = match active {
            SidebarPanel::Files => "EXPLORER",
            SidebarPanel::Search => "SEARCH",
            SidebarPanel::Structure => "STRUCTURE",
        };
        painter.text(
            egui::pos2(title_rect.left() + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            theme::TEXT_SECONDARY,
        );
        // 内容区
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), title_rect.bottom()),
            rect.right_bottom(),
        );
        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
        match active {
            SidebarPanel::Files => self.render_tree(&mut body_ui),
            SidebarPanel::Search => self.render_search(&mut body_ui),
            SidebarPanel::Structure => structure::render(&mut body_ui),
        }
    }

    fn render_tree(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("file_tree")
            .show(ui, |ui| {
                ui.add_space(4.0);
                for i in 0..self.tree_nodes.len() {
                    let selected = i == self.selected_index;
                    if tree::tree_row(ui, &self.tree_nodes[i], selected) {
                        self.selected_index = i;
                    }
                }
                ui.add_space(4.0);
            });
    }

    fn render_search(&self, ui: &mut egui::Ui) {
        search::search_box(ui);
        egui::ScrollArea::vertical()
            .id_salt("search_results")
            .show(ui, |ui| {
                ui.add_space(4.0);
                for result in &self.search_results {
                    search::search_row(ui, result);
                }
            });
    }
}
