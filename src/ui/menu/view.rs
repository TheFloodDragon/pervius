//! View 子菜单
//!
//! @author sky

use crate::app::App;
use crate::appearance::theme;
use crate::ui::editor::view_toggle::ActiveView;
use eframe::egui;
use egui_shell::components::menu_item;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, app: &mut App) {
    let mt = &theme::menu_theme();
    if menu_item(ui, mt, &t!("menu.decompiled"), None) {
        app.layout.editor.set_focused_view(ActiveView::Decompiled);
    }
    if menu_item(ui, mt, &t!("menu.bytecode"), None) {
        app.layout.editor.set_focused_view(ActiveView::Bytecode);
    }
    if menu_item(ui, mt, &t!("menu.hex"), None) {
        app.layout.editor.set_focused_view(ActiveView::Hex);
    }
    ui.separator();
    if menu_item(
        ui,
        mt,
        &t!("menu.toggle_explorer"),
        Some(&app.settings.keymap.toggle_explorer),
    ) {
        app.layout.explorer_visible = !app.layout.explorer_visible;
    }
}
