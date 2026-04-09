//! File 子菜单
//!
//! @author sky

use super::item::{menu_item, menu_item_raw, menu_submenu};
use crate::ui::keybindings;
use crate::ui::layout::Layout;
use eframe::egui;
use std::path::PathBuf;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    if menu_item(ui, "Open JAR...", Some(&keybindings::OPEN_JAR)) {
        layout.open_jar_dialog();
    }
    // Open Recent submenu
    let recent = layout.settings.recent.clone();
    let mut open_path: Option<PathBuf> = None;
    let mut clear = false;
    menu_submenu(ui, "Open Recent", |ui| {
        if recent.is_empty() {
            ui.add_enabled(
                false,
                egui::Label::new(
                    egui::RichText::new("No Recent Files")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                ),
            );
        } else {
            for entry in &recent {
                let dir = std::path::Path::new(&entry.path)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if menu_item_raw(ui, &entry.name, &dir) {
                    open_path = Some(PathBuf::from(&entry.path));
                }
            }
            ui.separator();
            if menu_item_raw(ui, "Clear Recent Files", "") {
                clear = true;
            }
        }
    });
    if let Some(path) = open_path {
        layout.open_jar(&path);
    }
    if clear {
        layout.settings.clear_recent();
        let _ = layout.settings.save();
    }
    ui.separator();
    if menu_item(
        ui,
        "Export Decompiled...",
        Some(&keybindings::EXPORT_DECOMPILED),
    ) {}
    if menu_item(ui, "Re-decompile", None) {
        layout.re_decompile();
    }
    ui.separator();
    if menu_item(ui, "Settings...", None) {
        layout.open_settings();
    }
    ui.separator();
    if menu_item_raw(ui, "Exit", "Alt+F4") {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
