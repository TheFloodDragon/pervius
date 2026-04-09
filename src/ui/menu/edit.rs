//! Edit 子菜单
//!
//! @author sky

use super::item::{menu_item, menu_item_raw};
use super::MenuAction;
use crate::ui::keybindings;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, cb: &mut dyn FnMut(MenuAction)) {
    menu_item_raw(ui, "Copy", "Ctrl+C");
    menu_item_raw(ui, "Paste", "Ctrl+V");
    menu_item_raw(ui, "Select All", "Ctrl+A");
    ui.separator();
    if menu_item(ui, "Find...", Some(&keybindings::FIND)) {
        cb(MenuAction::Find);
    }
    if menu_item(ui, "Find in Files...", Some(&keybindings::FIND_IN_FILES)) {
        cb(MenuAction::FindInFiles);
    }
}
