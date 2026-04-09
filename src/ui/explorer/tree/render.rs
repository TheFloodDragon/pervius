//! 文件树渲染
//!
//! @author sky

use super::node::TreeNode;
use crate::shell::{codicon, theme};
use crate::ui::menu::item::menu_item_raw;
use eframe::egui;
use egui_animation::Anim;
use std::collections::HashSet;

/// 递归渲染树节点，返回被点击的文件条目路径
///
/// `visible` 为后台线程计算的可见路径集合。
/// 集合非空时进入过滤模式（隐藏不在集合中的节点，自动展开子树）。
pub fn render_tree(
    ui: &mut egui::Ui,
    nodes: &mut [TreeNode],
    depth: u8,
    selected: &Option<String>,
    visible: &HashSet<String>,
    reveal: &mut Option<String>,
    scroll: bool,
) -> Option<String> {
    let filtering = !visible.is_empty();
    let mut clicked = None;
    for node in nodes.iter_mut() {
        if filtering && !visible.contains(&node.path) {
            continue;
        }
        let is_selected = selected.as_ref().is_some_and(|s| s == &node.path);
        let (single, double) = render_row(ui, node, depth, is_selected, reveal, scroll);
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
            if let Some(path) = render_tree(
                ui,
                &mut node.children,
                depth + 1,
                selected,
                visible,
                reveal,
                scroll,
            ) {
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
    scroll: bool,
) -> (bool, bool) {
    let row_h = 22.0;
    let indent_px = 8.0 + depth as f32 * 16.0;
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, row_h), egui::Sense::click());
    let painter = ui.painter();
    // 选中 / hover 背景动画
    let anim = Anim::new(ui, 0.1).with(&node.path);
    let target_bg = if selected {
        theme::verdigris_alpha(38)
    } else if response.hovered() {
        theme::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    let bg = anim.color("bg", target_bg);
    if bg.a() > 0 {
        painter.rect_filled(rect, 4.0, bg);
    }
    if selected && scroll {
        response.scroll_to_me(Some(egui::Align::Center));
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
