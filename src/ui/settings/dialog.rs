//! 设置对话框：FloatingWindow 承载，左侧分类侧栏 + 右侧设置项
//!
//! @author sky

use super::widget;
use crate::settings::Settings;
use crate::shell::{codicon, theme};
use eframe::egui;
use egui_window::FloatingWindow;

/// 侧栏分类
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Java,
}

impl Section {
    const ALL: &[Self] = &[Self::Java];

    fn label(self) -> &'static str {
        match self {
            Self::Java => "Java",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Java => codicon::BEAKER,
        }
    }
}

/// 侧栏宽度
const SIDEBAR_W: f32 = 130.0;
/// 侧栏项高度
const SIDEBAR_ITEM_H: f32 = 30.0;
/// 活跃项左侧 accent bar 宽度
const ACCENT_BAR_W: f32 = 2.0;

/// 设置对话框
pub struct SettingsDialog {
    window: FloatingWindow,
    section: Section,
    /// 编辑中的工作副本
    draft: Settings,
    /// 打开时的快照（用于检测变更）
    snapshot: Settings,
}

impl SettingsDialog {
    pub fn new() -> Self {
        Self {
            window: FloatingWindow::new("settings_window", "Settings")
                .icon('\u{EB51}')
                .default_size([520.0, 380.0])
                .min_size([420.0, 280.0]),
            section: Section::Java,
            draft: Settings::default(),
            snapshot: Settings::default(),
        }
    }

    /// 打开对话框，传入当前生效配置作为编辑起点
    pub fn open(&mut self, current: &Settings) {
        if self.window.is_open() {
            return;
        }
        self.draft = current.clone();
        self.snapshot = current.clone();
        self.section = Section::Java;
        self.window.open();
    }

    /// 每帧渲染，返回 `Some(settings)` 表示有变更需要应用
    pub fn render(&mut self, ctx: &egui::Context) -> Option<Settings> {
        let wt = theme::window_theme();
        let mut window = std::mem::take(&mut self.window);
        let mut changed = false;
        window.show(
            ctx,
            &wt,
            |_ui| {},
            |ui| {
                let total = ui.available_rect_before_wrap();
                // 侧栏宽度 clamp：不超过总宽度的一半
                let sw = SIDEBAR_W.min(total.width() * 0.5);
                // 侧栏背景
                let sidebar_rect =
                    egui::Rect::from_min_size(total.left_top(), egui::vec2(sw, total.height()));
                ui.painter()
                    .rect_filled(sidebar_rect, 0.0, theme::BG_DARKEST);
                // 侧栏内容
                let mut sidebar = ui.new_child(
                    egui::UiBuilder::new()
                        .id(egui::Id::new("settings_sidebar"))
                        .max_rect(sidebar_rect),
                );
                sidebar.set_clip_rect(sidebar_rect);
                self.render_sidebar(&mut sidebar);
                // 垂直分隔线
                ui.painter().line_segment(
                    [
                        egui::pos2(sidebar_rect.right(), total.top()),
                        egui::pos2(sidebar_rect.right(), total.bottom()),
                    ],
                    egui::Stroke::new(1.0, theme::BORDER),
                );
                // 右侧内容区（保证正宽度）
                let content_left = (sidebar_rect.right() + 1.0).min(total.right());
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(content_left, total.top()),
                    total.right_bottom(),
                );
                if content_rect.width() > 1.0 {
                    let mut content = ui.new_child(
                        egui::UiBuilder::new()
                            .id(egui::Id::new("settings_content"))
                            .max_rect(content_rect),
                    );
                    content.set_clip_rect(content_rect);
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .show(&mut content, |ui| {
                            changed = self.render_section(ui);
                        });
                }
                // 消耗整个 total 区域避免布局坍塌
                ui.allocate_rect(total, egui::Sense::hover());
            },
        );
        self.window = window;
        if !self.window.is_open() && self.has_changes() {
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

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        for &sec in Section::ALL {
            let active = self.section == sec;
            let avail_w = ui.available_width();
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(avail_w, SIDEBAR_ITEM_H), egui::Sense::click());
            let painter = ui.painter();
            if active {
                painter.rect_filled(rect, 0.0, theme::BG_HOVER);
                // 左侧 accent bar
                let bar = egui::Rect::from_min_size(
                    rect.left_top(),
                    egui::vec2(ACCENT_BAR_W, rect.height()),
                );
                painter.rect_filled(bar, 0.0, theme::VERDIGRIS);
            } else if resp.hovered() {
                painter.rect_filled(rect, 0.0, theme::BG_LIGHT);
            }
            let mid_y = rect.center().y;
            let icon_color = if active {
                theme::VERDIGRIS
            } else {
                theme::TEXT_MUTED
            };
            let text_color = if active {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };
            painter.text(
                egui::pos2(rect.left() + 14.0, mid_y),
                egui::Align2::LEFT_CENTER,
                sec.icon(),
                egui::FontId::new(13.0, codicon::family()),
                icon_color,
            );
            painter.text(
                egui::pos2(rect.left() + 34.0, mid_y),
                egui::Align2::LEFT_CENTER,
                sec.label(),
                egui::FontId::proportional(12.0),
                text_color,
            );
            if resp.clicked() {
                self.section = sec;
            }
        }
    }

    fn render_section(&mut self, ui: &mut egui::Ui) -> bool {
        match self.section {
            Section::Java => self.render_java(ui),
        }
    }

    fn render_java(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        widget::section_header(ui, "ENVIRONMENT");
        changed |= widget::path_picker(
            ui,
            "Java Home",
            &mut self.draft.java.java_home,
            "Use JAVA_HOME environment variable",
        );
        changed
    }
}
