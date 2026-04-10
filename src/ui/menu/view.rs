//! View 子菜单
//!
//! @author sky

use crate::appearance::theme;
use crate::ui::editor::view_toggle::ActiveView;
use crate::ui::layout::Layout;
use eframe::egui;
use egui_shell::components::menu_item;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    let mt = &theme::menu_theme();
    if menu_item(ui, mt, &t!("menu.decompiled"), None) {
        layout.editor.set_focused_view(ActiveView::Decompiled);
    }
    if menu_item(ui, mt, &t!("menu.bytecode"), None) {
        layout.editor.set_focused_view(ActiveView::Bytecode);
    }
    if menu_item(ui, mt, &t!("menu.hex"), None) {
        layout.editor.set_focused_view(ActiveView::Hex);
    }
    ui.separator();
    if menu_item(
        ui,
        mt,
        &t!("menu.toggle_explorer"),
        Some(&layout.settings.keymap.toggle_explorer),
    ) {
        layout.explorer_visible = !layout.explorer_visible;
    }
}
