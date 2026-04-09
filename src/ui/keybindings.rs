//! 应用快捷键定义：常量 + 回调，一处定义全局共用
//!
//! @author sky

use super::layout::Layout;
use eframe::egui;
use egui_keybind::{KeyBind, KeyMap};

// -- 快捷键常量（单一事实来源，菜单 / placeholder 共用）--

pub const TOGGLE_EXPLORER: KeyBind = KeyBind::alt(egui::Key::Num1);
pub const FIND_IN_FILES: KeyBind = KeyBind::ctrl_shift(egui::Key::F);
pub const OPEN_JAR: KeyBind = KeyBind::ctrl(egui::Key::O);
pub const EXPORT_DECOMPILED: KeyBind = KeyBind::ctrl_shift(egui::Key::E);
pub const FIND: KeyBind = KeyBind::ctrl(egui::Key::F);
pub const CLOSE_TAB: KeyBind = KeyBind::ctrl(egui::Key::W);
pub const CLOSE_ALL_TABS: KeyBind = KeyBind::ctrl_alt(egui::Key::W);

/// 构建应用级 KeyMap（快捷键 + 回调一并注册）
pub fn build_keymap() -> KeyMap<Layout> {
    KeyMap::new()
        .bind(TOGGLE_EXPLORER, |l: &mut Layout| {
            l.explorer_visible = !l.explorer_visible
        })
        .bind(OPEN_JAR, |l: &mut Layout| l.open_jar_dialog())
        .bind(CLOSE_TAB, |l: &mut Layout| l.editor.close_active_tab())
        .bind(CLOSE_ALL_TABS, |l: &mut Layout| l.editor.close_all_tabs())
        .bind_double_shift(|l: &mut Layout| l.search.open())
}
