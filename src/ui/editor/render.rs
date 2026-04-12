//! 编辑器视图渲染：代码视图 + Hex 视图
//!
//! 代码视图委托给 egui-editor，此处提供 EditorTab 解包的薄包装。
//!
//! @author sky

use super::tab::EditorTab;
use crate::appearance::theme;
use eframe::egui;
use egui_editor::code_view::NavigationHit;
use egui_editor::search::FindMatch;
use std::collections::HashSet;

pub use egui_editor::code_view::line_number_width;

/// 在 ScrollArea 外绘制全高背景（左侧 gutter + 右侧编辑区）
pub fn paint_editor_bg(ui: &egui::Ui, full_rect: egui::Rect, gutter_w: f32) {
    let t = theme::editor_theme();
    egui_editor::code_view::paint_editor_bg(ui, full_rect, gutter_w, &t);
}

/// 反编译视图
///
/// 返回 Ctrl+Click 产生的导航请求（如有）
pub fn render_decompiled(
    ui: &mut egui::Ui,
    tab: &mut EditorTab,
    matches: &[FindMatch],
    current: Option<usize>,
    known_classes: &HashSet<String>,
) -> Option<NavigationHit> {
    let t = theme::editor_theme();
    egui_editor::code_view::code_view(
        ui,
        egui::Id::new(&tab.entry_path).with("decompiled"),
        &tab.decompiled,
        &tab.decompiled_data.spans,
        &tab.decompiled_line_mapping,
        matches,
        current,
        &t,
        &mut tab.layout_cache,
        &mut tab.pending_scroll_to_line,
        Some(known_classes),
    )
}

/// 可编辑文本视图
///
/// 文本变更后自动刷新语法高亮数据并标记 tab 为已修改。
pub fn render_editable(
    ui: &mut egui::Ui,
    tab: &mut EditorTab,
    matches: &[FindMatch],
    current: Option<usize>,
) {
    let t = theme::editor_theme();
    let changed = egui_editor::code_view::code_view_editable(
        ui,
        egui::Id::new(&tab.entry_path).with("editable"),
        &mut tab.decompiled,
        tab.language,
        matches,
        current,
        &t,
        &mut tab.editable_layout_cache,
        tab.viewport_override,
        &mut tab.pending_scroll_to_line,
    );
    if changed {
        tab.refresh_decompiled_data();
        tab.is_modified = true;
    }
}

/// Hex 视图
pub fn render_hex(
    ui: &mut egui::Ui,
    tab: &mut EditorTab,
    matches: &[FindMatch],
    current: Option<usize>,
) {
    let hex_theme = super::style::hex::hex_theme();
    let highlights: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    egui_hex_view::show(
        ui,
        &tab.raw_bytes,
        &mut tab.hex_state,
        &hex_theme,
        &highlights,
        current,
    );
}
