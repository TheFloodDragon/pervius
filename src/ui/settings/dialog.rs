//! 设置对话框：SettingsPanel 承载，左侧分类侧栏 + 右侧设置项
//!
//! @author sky

use crate::settings::{KeymapSettings, Settings};
use crate::shell::{codicon, theme};
use crate::ui::keybindings;
use eframe::egui;
use egui_window_settings::{
    keybind_row, path_picker, section_header, sidebar_item, SettingsPanel, SettingsTheme,
};

/// 侧栏分类
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Java,
    Keymap,
}

impl Section {
    const ALL: &[Self] = &[Self::Java, Self::Keymap];

    fn label(self) -> &'static str {
        match self {
            Self::Java => "Java",
            Self::Keymap => "Keymap",
        }
    }

    fn icon(self) -> &'static str {
        match self {
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
            panel: SettingsPanel::new("settings_window", "Settings")
                .icon('\u{EB51}')
                .default_size([700.0, 500.0])
                .min_size([520.0, 380.0]),
            section: Section::Java,
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
        self.section = Section::Java;
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
                if sidebar_item(sidebar_ui, &st, sec.icon(), sec.label(), *section == sec) {
                    *section = sec;
                }
            }
            changed = render_section(*section, draft, content_ui, &st);
        });
        self.panel = panel;
        if !self.panel.is_open() && self.has_changes() {
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
        Section::Java => render_java(draft, ui, st),
        Section::Keymap => render_keymap(&mut draft.keymap, ui, st),
    }
}

fn render_java(draft: &mut Settings, ui: &mut egui::Ui, st: &SettingsTheme) -> bool {
    let mut changed = false;
    section_header(ui, st, "ENVIRONMENT");
    changed |= path_picker(
        ui,
        st,
        "Java Home",
        &mut draft.java.java_home,
        "Use JAVA_HOME environment variable",
        || {
            rfd::FileDialog::new()
                .pick_folder()
                .map(|p| p.to_string_lossy().into_owned())
        },
    );
    changed
}

fn render_keymap(km: &mut KeymapSettings, ui: &mut egui::Ui, st: &SettingsTheme) -> bool {
    let mut changed = false;
    let defaults = KeymapSettings::default();
    section_header(ui, st, "GENERAL");
    changed |= keybind_row(ui, st, "Open JAR", &mut km.open_jar, defaults.open_jar);
    changed |= keybind_row(
        ui,
        st,
        "Toggle Explorer",
        &mut km.toggle_explorer,
        defaults.toggle_explorer,
    );
    changed |= keybind_row(
        ui,
        st,
        "Open Settings",
        &mut km.open_settings,
        defaults.open_settings,
    );
    section_header(ui, st, "EDITOR");
    changed |= keybind_row(ui, st, "Find", &mut km.find, defaults.find);
    changed |= keybind_row(
        ui,
        st,
        "Find in Files",
        &mut km.find_in_files,
        defaults.find_in_files,
    );
    changed |= keybind_row(ui, st, "Close Tab", &mut km.close_tab, defaults.close_tab);
    changed |= keybind_row(
        ui,
        st,
        "Close All Tabs",
        &mut km.close_all_tabs,
        defaults.close_all_tabs,
    );
    changed |= keybind_row(
        ui,
        st,
        "Cycle View",
        &mut km.cycle_view,
        defaults.cycle_view,
    );
    section_header(ui, st, "EXPORT");
    changed |= keybind_row(
        ui,
        st,
        "Export Decompiled",
        &mut km.export_decompiled,
        defaults.export_decompiled,
    );
    changed
}
