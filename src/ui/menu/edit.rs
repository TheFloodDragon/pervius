//! Edit 子菜单
//!
//! @author sky

use crate::app::App;
use crate::appearance::theme;
use eframe::egui;
use egui_shell::components::menu_item;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, app: &mut App) {
    let mt = &theme::menu_theme();
    if menu_item(ui, mt, &t!("menu.save"), Some(&app.settings.keymap.save)) {
        app.save_active_tab();
    }
    ui.separator();
    if menu_item(ui, mt, &t!("menu.find"), Some(&app.settings.keymap.find)) {
        app.layout.editor.open_find();
    }
    if menu_item(
        ui,
        mt,
        &t!("menu.find_in_files"),
        Some(&app.settings.keymap.find_in_files),
    ) {
        app.layout.search.open();
    }
}
