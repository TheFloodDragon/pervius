//! 菜单栏：File / Edit / View / Help
//!
//! @author sky

mod edit;
mod file;
mod help;
pub mod item;
mod view;

use super::layout::Layout;
use crate::appearance::theme;
use crate::ui::widget::FlatButton;
use eframe::egui;
use rust_i18n::t;

/// 渲染菜单栏（注入到标题栏）
pub fn menu_bar(ui: &mut egui::Ui, layout: &mut Layout) {
    let menus: &[(&str, fn(&mut egui::Ui, &mut Layout))] = &[
        (&t!("menu.file"), file::render),
        (&t!("menu.edit"), edit::render),
        (&t!("menu.view"), view::render),
        (&t!("menu.help"), help::render),
    ];
    for (name, render) in menus {
        let btn = ui.add(FlatButton::new(*name).min_size(egui::vec2(40.0, 24.0)));
        egui::Popup::menu(&btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                render(ui, layout);
            });
    }
}
