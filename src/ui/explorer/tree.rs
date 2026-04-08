//! 文件树节点数据 + 单行渲染
//!
//! @author sky

use crate::shell::{codicon, theme};
use eframe::egui;

/// 文件树节点
pub struct TreeNode {
    pub label: String,
    pub indent: u8,
    pub is_folder: bool,
    pub is_expanded: bool,
    pub icon: &'static str,
}

/// 渲染一行树节点，返回是否被点击
pub fn tree_row(ui: &mut egui::Ui, node: &TreeNode, selected: bool) -> bool {
    let row_h = 26.0;
    let indent_px = 8.0 + node.indent as f32 * 16.0;
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
    let painter = ui.painter();
    let hovered = response.hovered();
    // 背景
    if selected {
        painter.rect_filled(rect, 4.0, theme::verdigris_alpha(38));
    } else if hovered {
        painter.rect_filled(rect, 4.0, theme::BG_HOVER);
    }
    let y = rect.center().y;
    let mut x = rect.left() + indent_px;
    // 折叠箭头
    if node.is_folder {
        let arrow = if node.is_expanded {
            codicon::CHEVRON_DOWN
        } else {
            codicon::CHEVRON_RIGHT
        };
        painter.text(
            egui::pos2(x + 6.0, y),
            egui::Align2::CENTER_CENTER,
            arrow,
            egui::FontId::new(10.0, codicon::family()),
            theme::TEXT_MUTED,
        );
    }
    x += 12.0 + 6.0;
    // 图标
    painter.text(
        egui::pos2(x + 8.0, y),
        egui::Align2::CENTER_CENTER,
        node.icon,
        egui::FontId::new(14.0, codicon::family()),
        theme::VERDIGRIS,
    );
    x += 16.0 + 6.0;
    // 名称
    let name_color = if selected || node.is_folder {
        theme::TEXT_PRIMARY
    } else {
        theme::VERDIGRIS
    };
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        &node.label,
        egui::FontId::proportional(12.0),
        name_color,
    );
    response.clicked()
}
