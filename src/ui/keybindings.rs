//! 应用快捷键定义：默认值 + KeyMap 构建
//!
//! @author sky

use super::layout::Layout;
use crate::settings::KeymapSettings;
use eframe::egui;
use egui_keybind::{KeyBind, KeyMap};

// -- 默认快捷键（用于 KeymapSettings::default 和编辑器占位符）--

pub const DEFAULT_TOGGLE_EXPLORER: KeyBind = KeyBind::alt(egui::Key::Num1);
pub const DEFAULT_FIND_IN_FILES: KeyBind = KeyBind::ctrl_shift(egui::Key::F);
pub const DEFAULT_OPEN_JAR: KeyBind = KeyBind::ctrl(egui::Key::O);
pub const DEFAULT_EXPORT_DECOMPILED: KeyBind = KeyBind::ctrl_shift(egui::Key::E);
pub const DEFAULT_FIND: KeyBind = KeyBind::ctrl(egui::Key::F);
pub const DEFAULT_SAVE: KeyBind = KeyBind::ctrl(egui::Key::S);
pub const DEFAULT_CLOSE_TAB: KeyBind = KeyBind::ctrl(egui::Key::W);
pub const DEFAULT_CLOSE_ALL_TABS: KeyBind = KeyBind::ctrl_alt(egui::Key::W);
pub const DEFAULT_CYCLE_VIEW: KeyBind = KeyBind::key(egui::Key::Tab);
pub const DEFAULT_OPEN_SETTINGS: KeyBind = KeyBind::ctrl(egui::Key::Comma);

/// 根据用户配置构建 KeyMap
pub fn build_keymap(km: &KeymapSettings) -> KeyMap<Layout> {
    KeyMap::new()
        .bind(km.toggle_explorer, |l: &mut Layout| {
            l.explorer_visible = !l.explorer_visible
        })
        .bind(km.open_jar, |l: &mut Layout| l.request_open_jar_dialog())
        .bind(km.close_tab, |l: &mut Layout| l.editor.close_active_tab())
        .bind(km.close_all_tabs, |l: &mut Layout| {
            l.editor.close_all_tabs()
        })
        .bind(km.find, |l: &mut Layout| l.editor.open_find())
        .bind(km.save, |l: &mut Layout| l.save_active_tab())
        .bind(km.cycle_view, |l: &mut Layout| l.editor.cycle_view())
        .bind(km.open_settings, |l: &mut Layout| l.open_settings())
        .bind_double_shift(|l: &mut Layout| l.search.open())
}
