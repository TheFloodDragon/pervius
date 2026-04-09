//! Help 子菜单
//!
//! @author sky

use super::item::menu_item;
use super::MenuAction;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, cb: &mut dyn FnMut(MenuAction)) {
    if menu_item(ui, "About Pervius", None) {
        cb(MenuAction::About);
    }
}
