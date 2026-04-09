//! File 子菜单
//!
//! @author sky

use super::item::{menu_item, menu_item_raw};
use super::MenuAction;
use crate::ui::keybindings;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, cb: &mut dyn FnMut(MenuAction)) {
    if menu_item(ui, "Open JAR...", Some(&keybindings::OPEN_JAR)) {
        cb(MenuAction::OpenJar);
    }
    if menu_item(ui, "Open Recent", None) {
        cb(MenuAction::OpenRecent);
    }
    ui.separator();
    if menu_item(
        ui,
        "Export Decompiled...",
        Some(&keybindings::EXPORT_DECOMPILED),
    ) {
        cb(MenuAction::ExportDecompiled);
    }
    ui.separator();
    if menu_item_raw(ui, "Exit", "Alt+F4") {
        cb(MenuAction::Exit);
    }
}
