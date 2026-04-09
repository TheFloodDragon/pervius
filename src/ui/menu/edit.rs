//! Edit 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::layout::Layout;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, "Find...", Some(&layout.settings.keymap.find)) {}
    if menu_item(
        ui,
        "Find in Files...",
        Some(&layout.settings.keymap.find_in_files),
    ) {}
}
