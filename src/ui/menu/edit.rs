//! Edit 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::layout::Layout;
use eframe::egui;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, &t!("menu.find"), Some(&layout.settings.keymap.find)) {}
    if menu_item(
        ui,
        &t!("menu.find_in_files"),
        Some(&layout.settings.keymap.find_in_files),
    ) {}
}
