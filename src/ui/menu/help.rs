//! Help 子菜单
//!
//! @author sky

use super::item::menu_item;
use crate::ui::layout::Layout;
use eframe::egui;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, _layout: &mut Layout) {
    if menu_item(ui, &t!("menu.about"), None) {}
}
