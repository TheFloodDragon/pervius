//! 菜单栏：File / Edit / View / Help
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;

/// 渲染菜单栏（注入到标题栏）
pub fn menu_bar(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    for name in ["File", "Edit", "View", "Help"] {
        let btn = ui.add(
            egui::Button::new(egui::RichText::new(name).size(12.0))
                .corner_radius(3.0)
                .min_size(egui::vec2(40.0, 24.0)),
        );
        egui::Popup::menu(&btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                ui.set_min_width(180.0);
                ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                match name {
                    "File" => file_menu(ui, &ctx),
                    "Edit" => edit_menu(ui),
                    "View" => view_menu(ui),
                    "Help" => {
                        menu_item(ui, "About Pervius", "");
                    }
                    _ => {}
                }
            });
    }
}

fn file_menu(ui: &mut egui::Ui, ctx: &egui::Context) {
    menu_item(ui, "Open JAR...", "Ctrl+O");
    menu_item(ui, "Open Recent", "");
    ui.separator();
    menu_item(ui, "Export Decompiled...", "Ctrl+Shift+E");
    ui.separator();
    if menu_item(ui, "Exit", "Alt+F4") {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn edit_menu(ui: &mut egui::Ui) {
    menu_item(ui, "Copy", "Ctrl+C");
    menu_item(ui, "Paste", "Ctrl+V");
    menu_item(ui, "Select All", "Ctrl+A");
    ui.separator();
    menu_item(ui, "Find...", "Ctrl+F");
    menu_item(ui, "Find in Files...", "Ctrl+Shift+F");
}

fn view_menu(ui: &mut egui::Ui) {
    menu_item(ui, "Decompiled", "");
    menu_item(ui, "Bytecode", "");
    menu_item(ui, "Hex", "");
    ui.separator();
    menu_item(ui, "Toggle Explorer", "Ctrl+B");
}

/// 菜单项：label 靠左，快捷键靠右
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    let r = ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(12.0)
                .color(theme::TEXT_PRIMARY),
        );
        if !shortcut.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(shortcut)
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
            });
        }
    });
    r.response.interact(egui::Sense::click()).clicked()
}
