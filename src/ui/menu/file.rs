//! File 子菜单
//!
//! @author sky

use crate::appearance::theme;
use crate::ui::layout::Layout;
use eframe::egui;
use egui_shell::components::{menu_item, menu_item_raw, menu_submenu, SettingsFile};
use rust_i18n::t;
use std::path::PathBuf;

pub fn render(ui: &mut egui::Ui, layout: &mut Layout) {
    let mt = &theme::menu_theme();
    if menu_item(
        ui,
        mt,
        &t!("menu.open_jar"),
        Some(&layout.settings.keymap.open_jar),
    ) {
        layout.request_open_jar_dialog();
    }
    // Open Recent submenu
    let recent = layout.settings.recent.clone();
    let mut open_path: Option<PathBuf> = None;
    let mut clear = false;
    menu_submenu(ui, mt, &t!("menu.open_recent"), |ui| {
        if recent.is_empty() {
            ui.add_enabled(
                false,
                egui::Label::new(
                    egui::RichText::new(t!("menu.no_recent"))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                ),
            );
        } else {
            let mt = &theme::menu_theme();
            for entry in &recent {
                let dir = std::path::Path::new(&entry.path)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if menu_item_raw(ui, mt, &entry.name, &dir) {
                    open_path = Some(PathBuf::from(&entry.path));
                }
            }
            ui.separator();
            if menu_item_raw(ui, mt, &t!("menu.clear_recent"), "") {
                clear = true;
            }
        }
    });
    if let Some(path) = open_path {
        layout.request_open_jar(&path);
    }
    if clear {
        layout.settings.clear_recent();
        let _ = layout.settings.save();
    }
    ui.separator();
    if menu_item(
        ui,
        mt,
        &t!("menu.export_decompiled"),
        Some(&layout.settings.keymap.export_decompiled),
    ) {
        layout.export_decompiled();
    }
    if menu_item(ui, mt, &t!("menu.re_decompile"), None) {
        layout.re_decompile();
    }
    ui.separator();
    if menu_item(
        ui,
        mt,
        &t!("menu.settings"),
        Some(&layout.settings.keymap.open_settings),
    ) {
        layout.open_settings();
    }
    ui.separator();
    if menu_item_raw(ui, mt, &t!("menu.exit"), "Alt+F4") {
        layout.request_close(ui.ctx());
    }
}
