//! 侧边图标栏（48px 宽，竖排图标按钮）
//!
//! 对照 sidebar.slint：Files / Search / Structure 顶部排列，Settings 底部。
//! 激活项左侧有 2px 铜绿指示条。
//!
//! @author sky

use crate::shell::{codicon, theme};
use eframe::egui;

/// 侧边栏面板类型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    Files,
    Search,
    Structure,
}

/// 侧边栏状态
pub struct Sidebar {
    pub active: SidebarPanel,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            active: SidebarPanel::Files,
        }
    }
}

impl Sidebar {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::BG_DARKEST);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.vertical(|ui| {
                ui.add_space(4.0);
                self.icon_button(ui, codicon::FILES, "Files", SidebarPanel::Files);
                self.icon_button(ui, codicon::SEARCH, "Search", SidebarPanel::Search);
                self.icon_button(
                    ui,
                    codicon::SYMBOL_MISC,
                    "Structure",
                    SidebarPanel::Structure,
                );
                // 弹簧：撑开剩余空间
                ui.add_space(ui.available_height() - 48.0);
                self.icon_button_passive(ui, codicon::SETTINGS_GEAR, "Settings");
            });
        });
    }

    fn icon_button(&mut self, ui: &mut egui::Ui, icon: &str, label: &str, panel: SidebarPanel) {
        let is_active = self.active == panel;
        let clicked = Self::draw_icon(ui, icon, label, is_active);
        if clicked {
            self.active = panel;
        }
    }

    fn icon_button_passive(&self, ui: &mut egui::Ui, icon: &str, label: &str) {
        Self::draw_icon(ui, icon, label, false);
    }

    fn draw_icon(ui: &mut egui::Ui, icon: &str, label: &str, is_active: bool) -> bool {
        let size = egui::vec2(theme::SIDEBAR_WIDTH, 44.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let hovered = response.hovered();
        let painter = ui.painter();
        // hover 背景
        if hovered {
            painter.rect_filled(rect, 4.0, theme::BG_HOVER);
        }
        // 激活指示条
        if is_active {
            let bar = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
            painter.rect_filled(bar, 1.0, theme::VERDIGRIS);
        }
        // 图标
        let icon_color = if is_active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        painter.text(
            egui::pos2(rect.center().x, rect.center().y - 6.0),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::new(16.0, codicon::family()),
            icon_color,
        );
        // 标签文字
        let label_color = if is_active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        };
        painter.text(
            egui::pos2(rect.center().x, rect.center().y + 10.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(8.0),
            label_color,
        );
        response.clicked()
    }
}
