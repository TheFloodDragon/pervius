//! 菜单栏：File / Edit / View / Help
//!
//! @author sky

mod edit;
mod file;
mod help;
pub mod item;
mod view;

use crate::shell::theme;
use eframe::egui;

/// 菜单动作
pub enum MenuAction {
    OpenJar,
    OpenRecent,
    ExportDecompiled,
    Find,
    FindInFiles,
    ToggleExplorer,
    ViewDecompiled,
    ViewBytecode,
    ViewHex,
    About,
    Exit,
}

/// 渲染菜单栏（注入到标题栏）
pub fn menu_bar(ui: &mut egui::Ui, callback: &mut dyn FnMut(MenuAction)) {
    let menus: &[(&str, fn(&mut egui::Ui, &mut dyn FnMut(MenuAction)))] = &[
        ("File", file::render),
        ("Edit", edit::render),
        ("View", view::render),
        ("Help", help::render),
    ];
    for &(name, render) in menus {
        let btn = ui.add(
            egui::Button::new(egui::RichText::new(name).size(12.0))
                .corner_radius(3.0)
                .min_size(egui::vec2(40.0, 24.0)),
        );
        egui::Popup::menu(&btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                render(ui, callback);
            });
    }
}
