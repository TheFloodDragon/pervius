//! View 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::editor::view_toggle::ActiveView;
use crate::ui::layout::Layout;
use eframe::egui;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, &t!("menu.decompiled"), None) {
        layout.editor.set_focused_view(ActiveView::Decompiled);
    }
    if menu_item(ui, &t!("menu.bytecode"), None) {
        layout.editor.set_focused_view(ActiveView::Bytecode);
    }
    if menu_item(ui, &t!("menu.hex"), None) {
        layout.editor.set_focused_view(ActiveView::Hex);
    }
    ui.separator();
    if menu_item(
        ui,
        &t!("menu.toggle_explorer"),
        Some(&layout.settings.keymap.toggle_explorer),
    ) {
        layout.explorer_visible = !layout.explorer_visible;
    }
}
