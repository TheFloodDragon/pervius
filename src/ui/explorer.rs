//! 左侧面板：文件树 / 搜索
//!
//! @author sky

pub mod search;
pub mod tree;

use crate::shell::theme;
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

    /// 在给定 rect 内渲染（背景由 layout island 绘制）
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        // 面板标题
        let title_h = 32.0;
        let title_rect =
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(rect.width(), title_h));
        painter.text(
            egui::pos2(title_rect.left() + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "EXPLORER",
            egui::FontId::proportional(11.0),
            theme::TEXT_SECONDARY,
        );
        // 内容区（左右 2px padding）
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, title_rect.bottom()),
            egui::pos2(rect.right() - 2.0, rect.bottom()),
        );
        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
        self.render_tree(&mut body_ui);
    }

    fn render_tree(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("file_tree")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
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
}
