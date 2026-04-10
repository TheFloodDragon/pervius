//! 菜单栏：File / Edit / View / Help
//!
//! @author sky

mod edit;
mod file;
mod help;
mod view;

use crate::app::App;
use crate::appearance::theme;
use crate::ui::widget::{flat_button_theme, FlatButton};
use eframe::egui;
use rust_i18n::t;

/// 渲染菜单栏（注入到标题栏）
pub fn menu_bar(ui: &mut egui::Ui, app: &mut App) {
    let fbt = flat_button_theme();
    let menus: &[(&str, fn(&mut egui::Ui, &mut App))] = &[
        (&t!("menu.file"), file::render),
        (&t!("menu.edit"), edit::render),
        (&t!("menu.view"), view::render),
        (&t!("menu.help"), help::render),
    ];
    for (name, render) in menus {
        let btn = ui.add(FlatButton::new(*name, &fbt).min_size(egui::vec2(40.0, 24.0)));
        egui::Popup::menu(&btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                render(ui, app);
            });
    }
}
