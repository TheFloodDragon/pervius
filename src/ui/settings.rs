//! 设置对话框：SettingsPanel 承载，左侧分类侧栏 + 右侧设置项
//!
//! @author sky

use crate::settings::{KeymapSettings, Language, Settings};
use crate::shell::{codicon, theme};
use eframe::egui;
use egui_shell::components::{
    path_picker_with, section_header, sidebar_item, SettingsPanel, SettingsTheme,
};
use egui_shell::keybind_rows;
use rust_i18n::t;

/// 侧栏分类
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    General,
    Java,
    Keymap,
}

impl Section {
    const ALL: &[Self] = &[Self::General, Self::Java, Self::Keymap];

    fn label(self) -> String {
        match self {
            Self::General => t!("settings.general").to_string(),
            Self::Java => t!("settings.java").to_string(),
            Self::Keymap => t!("settings.keymap").to_string(),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::General => codicon::SETTINGS_GEAR,
            Self::Java => codicon::BEAKER,
            Self::Keymap => codicon::KEYBOARD,
        }
    }
}

/// 设置对话框
pub struct SettingsDialog {
    panel: SettingsPanel,
    section: Section,
    /// 编辑中的工作副本
    draft: Settings,
    /// 打开时的快照（用于检测变更）
    snapshot: Settings,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            panel: SettingsPanel::new("settings_window", t!("settings.title").to_string())
                .icon(codicon::SETTINGS_GEAR)
                .default_size([700.0, 500.0])
                .min_size([520.0, 380.0]),
            section: Section::General,
            draft: Settings::default(),
            snapshot: Settings::default(),
        }
    }

    /// 打开对话框，传入当前生效配置作为编辑起点
    pub fn open(&mut self, current: &Settings) {
        if self.panel.is_open() {
            return;
        }
        self.draft = current.clone();
        self.snapshot = current.clone();
        self.section = Section::General;
        self.panel.open();
    }

    /// 每帧渲染，返回 `Some(settings)` 表示有变更需要应用
    pub fn render(&mut self, ctx: &egui::Context) -> Option<Settings> {
        let wt = theme::window_theme();
        let st = theme::settings_theme();
        let mut panel = std::mem::take(&mut self.panel);
        let mut changed = false;
        let section = &mut self.section;
        let draft = &mut self.draft;
        panel.show(ctx, &wt, &st, |sidebar_ui, content_ui| {
            sidebar_ui.add_space(6.0);
            for &sec in Section::ALL {
                if sidebar_item(sidebar_ui, &st, sec.icon(), &sec.label(), *section == sec) {
                    *section = sec;
                }
            }
            changed = render_section(*section, draft, content_ui, &st);
        });
        self.panel = panel;
        if !self.panel.is_open() && self.has_changes() {
            self.snapshot = self.draft.clone();
            return Some(self.draft.clone());
        }
        if changed {
            return Some(self.draft.clone());
        }
        None
    }

    fn has_changes(&self) -> bool {
        let a = toml::to_string(&self.draft).unwrap_or_default();
        let b = toml::to_string(&self.snapshot).unwrap_or_default();
        a != b
    }
}

fn render_section(
    section: Section,
    draft: &mut Settings,
    ui: &mut egui::Ui,
    st: &SettingsTheme,
) -> bool {
    match section {
        Section::General => render_general(draft, ui, st),
        Section::Java => render_java(draft, ui, st),
        Section::Keymap => render_keymap(&mut draft.keymap, ui, st),
    }
}

fn render_general(draft: &mut Settings, ui: &mut egui::Ui, st: &SettingsTheme) -> bool {
    let mut changed = false;
    section_header(ui, st, &t!("settings.section_general"));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(t!("settings.language").to_string())
                .size(13.0)
                .color(st.text_primary),
        );
        ui.add_space(8.0);
        let current = draft.language;
        egui::ComboBox::from_id_salt("language_combo")
            .selected_text(current.label())
            .width(120.0)
            .show_ui(ui, |ui| {
                for &lang in Language::ALL {
                    if ui.selectable_label(current == lang, lang.label()).clicked() {
                        draft.language = lang;
                        rust_i18n::set_locale(lang.code());
                        changed = true;
                    }
                }
            });
    });
    ui.add_space(4.0);
    changed
}

fn render_java(draft: &mut Settings, ui: &mut egui::Ui, st: &SettingsTheme) -> bool {
    let mut changed = false;
    section_header(ui, st, &t!("settings.environment"));
    changed |= path_picker_with(
        ui,
        st,
        &t!("settings.java_home"),
        &mut draft.java.java_home,
        &t!("settings.java_home_hint"),
        &t!("settings.browse"),
        || {
            rfd::FileDialog::new()
                .pick_folder()
                .map(|p| p.to_string_lossy().into_owned())
        },
    );
    changed
}

fn render_keymap(km: &mut KeymapSettings, ui: &mut egui::Ui, st: &SettingsTheme) -> bool {
    let defaults = KeymapSettings::default();
    let hint = t!("settings.press_key");
    let mut changed = false;
    section_header(ui, st, &t!("settings.section_general"));
    changed |= keybind_rows!(ui, st, hint, km, defaults,
        t!("settings.open_jar") => open_jar,
        t!("settings.toggle_explorer") => toggle_explorer,
        t!("settings.open_settings") => open_settings,
    );
    section_header(ui, st, &t!("settings.section_editor"));
    changed |= keybind_rows!(ui, st, hint, km, defaults,
        t!("settings.save") => save,
        t!("settings.find") => find,
        t!("settings.find_in_files") => find_in_files,
        t!("settings.close_tab") => close_tab,
        t!("settings.close_all_tabs") => close_all_tabs,
        t!("settings.cycle_view") => cycle_view,
    );
    section_header(ui, st, &t!("settings.section_export"));
    changed |= keybind_rows!(ui, st, hint, km, defaults,
        t!("settings.export_decompiled") => export_decompiled,
    );
    changed
}
