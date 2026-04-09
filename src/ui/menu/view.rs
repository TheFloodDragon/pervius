//! View 子菜单
//!
//! @author sky

use super::item::menu_item;
use super::MenuAction;
use crate::ui::keybindings;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, cb: &mut dyn FnMut(MenuAction)) {
    if menu_item(ui, "Decompiled", None) {
        cb(MenuAction::ViewDecompiled);
    }
    if menu_item(ui, "Bytecode", None) {
        cb(MenuAction::ViewBytecode);
    }
    if menu_item(ui, "Hex", None) {
        cb(MenuAction::ViewHex);
    }
    ui.separator();
    if menu_item(ui, "Toggle Explorer", Some(&keybindings::TOGGLE_EXPLORER)) {
        cb(MenuAction::ToggleExplorer);
    }
}
