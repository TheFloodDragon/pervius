//! View 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::layout::Layout;
use eframe::egui;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, &t!("menu.decompiled"), None) {}
    if menu_item(ui, &t!("menu.bytecode"), None) {}
    if menu_item(ui, &t!("menu.hex"), None) {}
    ui.separator();
    if menu_item(
        ui,
        &t!("menu.toggle_explorer"),
        Some(&layout.settings.keymap.toggle_explorer),
    ) {
        layout.explorer_visible = !layout.explorer_visible;
    }
}
