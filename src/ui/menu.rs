//! 菜单栏：File / Edit / View / Help
//!
//! @author sky

mod edit;
mod file;
mod help;
pub mod item;
mod view;

use super::layout::Layout;
use crate::shell::theme;
use crate::ui::widget::FlatButton;
use eframe::egui;

/// 渲染菜单栏（注入到标题栏）
pub fn menu_bar(ui: &mut egui::Ui, layout: &mut Layout) {
    let menus: &[(&str, fn(&mut egui::Ui, &mut Layout))] = &[
        ("File", file::render),
        ("Edit", edit::render),
        ("View", view::render),
        ("Help", help::render),
    ];
    for &(name, render) in menus {
        let btn = ui.add(FlatButton::new(name).min_size(egui::vec2(40.0, 24.0)));
        egui::Popup::menu(&btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                render(ui, layout);
            });
    }
}
