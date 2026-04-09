//! TabViewer 实现：定义每个 Tab 的标题和内容渲染
//!
//! @author sky

use super::render;
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use crate::shell::{codicon, theme};
use crate::ui::keybindings;
use crate::ui::menu::item::menu_item;
use eframe::egui;
use egui_dock::{NodePath, TabViewer};

/// Tab 右键菜单触发的动作
pub enum TabAction {
    /// 关闭当前 tab
    Close(Option<String>),
    CloseAll,
    /// 关闭除指定 tab 外的所有 tab（按 entry_path 匹配）
    CloseOthers(Option<String>),
    /// 关闭指定 tab 右侧的所有 tab
    CloseToRight(Option<String>),
}

pub struct EditorTabViewer {
    pub action: Option<TabAction>,
}

impl TabViewer for EditorTabViewer {
    type Tab = EditorTab;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match &tab.entry_path {
            Some(path) => egui::Id::new(path),
            None => egui::Id::new(&tab.title),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let mut job = egui::text::LayoutJob::default();
        job.append("", 4.0, egui::TextFormat::default());
        job.append(
            codicon::JAVA,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(11.0, codicon::family()),
                color: theme::VERDIGRIS,
                ..Default::default()
            },
        );
        job.append(" ", 0.0, egui::TextFormat::default());
        job.append(
            &tab.title,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: egui::Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        if tab.is_modified {
            job.append(" ", 0.0, egui::TextFormat::default());
            job.append(
                codicon::CIRCLE_FILLED,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(6.0),
                    color: theme::ACCENT_ORANGE,
                    ..Default::default()
                },
            );
        }
        job.append("", 4.0, egui::TextFormat::default());
        job.into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        match tab.active_view {
            ActiveView::Decompiled => render::render_decompiled(ui, tab),
            ActiveView::Bytecode => render::render_bytecode(ui, tab),
            ActiveView::Hex => render::render_hex(ui, tab),
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: NodePath) {
        if menu_item(ui, "Close", Some(&keybindings::CLOSE_TAB)) {
            self.action = Some(TabAction::Close(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(ui, "Close Others", None) {
            self.action = Some(TabAction::CloseOthers(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(ui, "Close to the Right", None) {
            self.action = Some(TabAction::CloseToRight(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(ui, "Close All", Some(&keybindings::CLOSE_ALL_TABS)) {
            self.action = Some(TabAction::CloseAll);
            ui.close();
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
