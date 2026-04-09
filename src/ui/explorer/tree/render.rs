//! 文件树渲染 + 过滤
//!
//! @author sky

use super::node::TreeNode;
use crate::shell::{codicon, theme};
use crate::ui::menu::item::menu_item_raw;
use eframe::egui;

/// 递归判断节点是否匹配过滤（自身标签或任意后代标签包含 filter）
fn matches_filter(node: &TreeNode, filter: &str) -> bool {
    if node.label.to_ascii_lowercase().contains(filter) {
        return true;
    }
    node.children.iter().any(|c| matches_filter(c, filter))
}

/// 查找第一个匹配过滤的文件节点路径（深度优先）
pub fn first_match(nodes: &[TreeNode], filter: &str) -> Option<String> {
    for node in nodes {
        if !node.is_folder && node.label.to_ascii_lowercase().contains(filter) {
            return Some(node.path.clone());
        }
        if let Some(path) = first_match(&node.children, filter) {
            return Some(path);
        }
    }
    None
}

/// 递归渲染树节点，返回被点击的文件条目路径
pub fn render_tree(
    ui: &mut egui::Ui,
    nodes: &mut [TreeNode],
    depth: u8,
    selected: &Option<String>,
    filter: &str,
    reveal: &mut Option<String>,
) -> Option<String> {
    let filtering = !filter.is_empty();
    let mut clicked = None;
    for node in nodes.iter_mut() {
        // 过滤模式：跳过无匹配的子树
        if filtering && !matches_filter(node, filter) {
            continue;
        }
        let is_selected = selected.as_ref().is_some_and(|s| s == &node.path);
        let (single, double) = render_row(ui, node, depth, is_selected, reveal);
        if node.is_folder {
            if single {
                node.expanded = !node.expanded;
            }
        } else if node.label.starts_with('$') {
            // 子类：双击展开，不打开文件
            if double {
                node.expanded = !node.expanded;
            }
        } else {
            if single {
                clicked = Some(node.path.clone());
            }
            if double && node.has_children() {
                node.expanded = !node.expanded;
            }
        }
        // 过滤模式自动展开所有匹配路径，正常模式遵循 expanded
        let show_children = node.has_children() && (filtering || node.expanded);
        if show_children {
            if let Some(path) =
                render_tree(ui, &mut node.children, depth + 1, selected, filter, reveal)
            {
                clicked = Some(path);
            }
        }
    }
    clicked
}

/// 渲染一行树节点，返回 (单击, 双击)
fn render_row(
    ui: &mut egui::Ui,
    node: &TreeNode,
    depth: u8,
    selected: bool,
    reveal: &mut Option<String>,
) -> (bool, bool) {
    let row_h = 22.0;
    let indent_px = 8.0 + depth as f32 * 16.0;
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 4.0, theme::verdigris_alpha(38));
    } else if response.hovered() {
        painter.rect_filled(rect, 4.0, theme::BG_HOVER);
    }
    let y = rect.center().y;
    let mut x = rect.left() + indent_px;
    // 折叠箭头（目录或有子节点的 class）
    if node.is_folder || node.has_children() {
        let arrow = if node.expanded {
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
        node.icon(),
        egui::FontId::new(14.0, codicon::family()),
        node.icon_color(),
    );
    x += 16.0 + 6.0;
    // 名称
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        &node.label,
        egui::FontId::proportional(12.0),
        theme::TEXT_PRIMARY,
    );
    // 右键菜单
    if !node.is_folder {
        response.context_menu(|ui| {
            ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
            if menu_item_raw(ui, "Reveal in Explorer", "") {
                *reveal = Some(node.path.clone());
                ui.close();
            }
        });
    }
    (response.clicked(), response.double_clicked())
}
