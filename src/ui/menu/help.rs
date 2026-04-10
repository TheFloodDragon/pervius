//! Help 子菜单
//!
//! @author sky

use crate::appearance::theme;
use crate::ui::layout::Layout;
use eframe::egui;
use egui_shell::components::menu_item;
use rust_i18n::t;

pub fn render(ui: &mut egui::Ui, _layout: &mut Layout) {
    let mt = &theme::menu_theme();
    if menu_item(ui, mt, &t!("menu.about"), None) {}
}
