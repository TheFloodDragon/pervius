//! 文件树渲染（虚拟滚动）
//!
//! 使用 ScrollArea::show_rows() 实现行级虚拟化，
//! 只渲染视口内可见的行，全展开 5000+ 节点也不掉帧。
//!
//! @author sky

use super::node::TreeNode;
use crate::shell::{codicon, theme};
use crate::ui::menu::item::menu_item_raw;
use eframe::egui;
use egui_animation::Anim;
use rust_i18n::t;
use std::collections::HashSet;
use std::ops::Range;

/// 行高
const ROW_H: f32 = 22.0;

/// 递归计数可见（展开）节点总数
fn count_rows(nodes: &[TreeNode], visible: &HashSet<String>) -> usize {
    let filtering = !visible.is_empty();
    let mut count = 0;
    for node in nodes {
        if filtering && !visible.contains(&node.path) {
            continue;
        }
        count += 1;
        if node.has_children() && (filtering || node.expanded) {
            count += count_rows(&node.children, visible);
        }
    }
    count
}

/// 虚拟滚动入口：计数 → show_rows → 只渲染可见范围
pub fn render_tree(
    ui: &mut egui::Ui,
    nodes: &mut [TreeNode],
    selected: &Option<String>,
    visible: &HashSet<String>,
    reveal: &mut Option<String>,
    scroll: bool,
    tab_modified: &HashSet<String>,
    jar_modified: &HashSet<String>,
    decompiled_classes: Option<&HashSet<String>>,
) -> Option<String> {
    ui.spacing_mut().item_spacing.y = 2.0;
    let row_h = ROW_H + 2.0;
    let total = count_rows(nodes, visible);
    let mut clicked = None;
    egui::ScrollArea::vertical()
        .id_salt("file_tree")
        .auto_shrink(false)
        .min_scrolled_height(ui.available_height())
        .show_rows(ui, row_h, total, |ui, range| {
            let mut counter = 0usize;
            clicked = render_range(
                ui,
                nodes,
                0,
                &mut counter,
                &range,
                selected,
                visible,
                reveal,
                scroll,
                tab_modified,
                jar_modified,
                decompiled_classes,
            );
        });
    clicked
}

/// 递归遍历树，只渲染 counter 落在 range 内的节点
fn render_range(
    ui: &mut egui::Ui,
    nodes: &mut [TreeNode],
    depth: u8,
    counter: &mut usize,
    range: &Range<usize>,
    selected: &Option<String>,
    visible: &HashSet<String>,
    reveal: &mut Option<String>,
    scroll: bool,
    tab_modified: &HashSet<String>,
    jar_modified: &HashSet<String>,
    decompiled_classes: Option<&HashSet<String>>,
) -> Option<String> {
    let filtering = !visible.is_empty();
    let mut clicked = None;
    for node in nodes.iter_mut() {
        if filtering && !visible.contains(&node.path) {
            continue;
        }
        let idx = *counter;
        *counter += 1;
        // 已超过可见范围，后续节点全部跳过
        if idx >= range.end {
            return clicked;
        }
        // 在可见范围内才渲染
        if idx >= range.start {
            let is_selected = selected.as_ref().is_some_and(|s| s == &node.path);
            let mod_color = if tab_modified.contains(&node.path) {
                Some(theme::ACCENT_ORANGE)
            } else if jar_modified.contains(&node.path) {
                Some(theme::ACCENT_GREEN)
            } else {
                None
            };
            let (single, double) = render_row(
                ui,
                node,
                depth,
                is_selected,
                mod_color,
                decompiled_classes,
                reveal,
                scroll,
            );
            if node.is_folder {
                if single {
                    node.expanded = !node.expanded;
                }
            } else if node.label.starts_with('$') {
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
        }
        // 子节点递归（无论当前节点是否在可见范围，都需要正确计数）
        let show_children = node.has_children() && (filtering || node.expanded);
        if show_children {
            if let Some(path) = render_range(
                ui,
                &mut node.children,
                depth + 1,
                counter,
                range,
                selected,
                visible,
                reveal,
                scroll,
                tab_modified,
                jar_modified,
                decompiled_classes,
            ) {
                clicked = Some(path);
            }
        }
    }
    clicked
}

/// 判断节点是否已反编译
///
/// `decompiled_classes` 为 None 表示全部已完成，Some 则逐个检查：
/// - 根节点（path 为空）：始终视为已完成
/// - 文件夹：路径在集合中（反编译线程写入类名时同时写入文件夹前缀）
/// - class 文件：去掉 `.class` 后缀和内部类 `$` 后缀，检查外层类是否在集合中
fn is_node_decompiled(node: &TreeNode, decompiled_classes: Option<&HashSet<String>>) -> bool {
    let set = match decompiled_classes {
        None => return true,
        Some(s) => s,
    };
    if node.path.is_empty() {
        return true;
    }
    if node.is_folder {
        return set.contains(&node.path);
    }
    let without_ext = node.path.strip_suffix(".class").unwrap_or(&node.path);
    let base = match without_ext.find('$') {
        Some(pos) => &without_ext[..pos],
        None => without_ext,
    };
    set.contains(base)
}

/// 渲染一行树节点，返回 (单击, 双击)
fn render_row(
    ui: &mut egui::Ui,
    node: &TreeNode,
    depth: u8,
    selected: bool,
    mod_color: Option<egui::Color32>,
    decompiled_classes: Option<&HashSet<String>>,
    reveal: &mut Option<String>,
    scroll: bool,
) -> (bool, bool) {
    let indent_px = 8.0 + depth as f32 * 16.0;
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, ROW_H), egui::Sense::click());
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
    // 折叠箭头
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
    // 按节点判断是否已反编译，未完成的暗显，完成后动画过渡到正常色
    let node_decompiled = is_node_decompiled(node, decompiled_classes);
    let decompile_anim = Anim::new(ui, 0.35).with(&node.path);
    let target_icon = if node_decompiled {
        node.icon_color()
    } else {
        theme::TEXT_MUTED
    };
    let icon_color = decompile_anim.color("icon", target_icon);
    let target_label = if let Some(c) = mod_color {
        c
    } else if node_decompiled {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };
    let label_color = decompile_anim.color("label", target_label);
    // 图标
    painter.text(
        egui::pos2(x + 8.0, y),
        egui::Align2::CENTER_CENTER,
        node.icon(),
        egui::FontId::new(14.0, codicon::family()),
        icon_color,
    );
    x += 16.0 + 6.0;
    // 名称
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        &node.label,
        egui::FontId::proportional(12.0),
        label_color,
    );
    // 右键菜单
    if !node.is_folder {
        response.context_menu(|ui| {
            ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
            if menu_item_raw(ui, &t!("explorer.reveal"), "") {
                *reveal = Some(node.path.clone());
                ui.close();
            }
        });
    }
    (response.clicked(), response.double_clicked())
}
