//! TabViewer 实现：定义每个 Tab 的标题和内容渲染
//!
//! @author sky

use super::find::FindBar;
use super::render;
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use crate::shell::{codicon, theme};
use crate::ui::keybindings;
use crate::ui::menu::item::menu_item;
use eframe::egui;
use egui_dock::{NodePath, TabViewer};
use rust_i18n::t;

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

pub struct EditorTabViewer<'a> {
    pub action: Option<TabAction>,
    pub find_bar: &'a mut FindBar,
}

impl TabViewer for EditorTabViewer<'_> {
    type Tab = EditorTab;

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
        let content_rect = ui.max_rect();
        // 更新搜索（先于内容渲染，提供高亮数据）
        if self.find_bar.open {
            self.find_bar.update(tab);
        }
        let (matches, current) = if self.find_bar.open {
            self.find_bar.highlight_info()
        } else {
            (vec![], None)
        };
        // 渲染内容
        match tab.active_view {
            ActiveView::Decompiled => render::render_decompiled(ui, tab, &matches, current),
            ActiveView::Bytecode => render::render_bytecode(ui, tab, &matches, current),
            ActiveView::Hex => render::render_hex(ui, tab),
        }
        // 浮动查找栏（overlay）
        if self.find_bar.open {
            self.find_bar.render_overlay(ui, content_rect);
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: NodePath) {
        if menu_item(
            ui,
            &t!("editor.close"),
            Some(&keybindings::DEFAULT_CLOSE_TAB),
        ) {
            self.action = Some(TabAction::Close(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(ui, &t!("editor.close_others"), None) {
            self.action = Some(TabAction::CloseOthers(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(ui, &t!("editor.close_to_right"), None) {
            self.action = Some(TabAction::CloseToRight(tab.entry_path.clone()));
            ui.close();
        }
        if menu_item(
            ui,
            &t!("editor.close_all"),
            Some(&keybindings::DEFAULT_CLOSE_ALL_TABS),
        ) {
            self.action = Some(TabAction::CloseAll);
            ui.close();
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match &tab.entry_path {
            Some(path) => egui::Id::new(path),
            None => egui::Id::new(&tab.title),
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
