//! View 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::layout::Layout;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, "Decompiled", None) {}
    if menu_item(ui, "Bytecode", None) {}
    if menu_item(ui, "Hex", None) {}
    ui.separator();
    if menu_item(
        ui,
        "Toggle Explorer",
        Some(&layout.settings.keymap.toggle_explorer),
    ) {
        layout.explorer_visible = !layout.explorer_visible;
    }
}
