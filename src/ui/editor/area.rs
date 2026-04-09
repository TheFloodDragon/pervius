//! 编辑器区域：egui_dock 容器 + tab 管理
//!
//! @author sky

use super::render::{self, line_number_width};
use super::style::dock;
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use super::viewer::EditorTabViewer;
use crate::shell::theme;
use eframe::egui;
use egui_dock::{DockArea, DockState};

/// 编辑器区域状态
pub struct EditorArea {
    pub dock_state: DockState<EditorTab>,
}

impl EditorArea {
    pub fn new(tabs: Vec<EditorTab>) -> Self {
        let dock_state = DockState::new(tabs);
        Self { dock_state }
    }

    /// 在给定 UI 区域内渲染
    pub fn render(&mut self, ui: &mut egui::Ui) {
        if self.is_empty() {
            Self::render_placeholder(ui);
            return;
        }
        let rect = ui.max_rect();
        // 在 DockArea（ScrollArea）外画全高 gutter 背景
        if let Some(gutter_w) = self.active_gutter_width() {
            render::paint_editor_bg(ui, rect, gutter_w);
        }
        let style = dock::build(ui.style());
        DockArea::new(&mut self.dock_state)
            .style(style)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_inside(ui, &mut EditorTabViewer);
    }

    fn is_empty(&self) -> bool {
        self.dock_state.main_surface().num_tabs() == 0
    }

    /// 无文件打开时的占位提示（IDEA 风格：操作名 + 快捷键）
    fn render_placeholder(ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        let center = rect.center();
        // (操作名, 快捷键)
        let hints: &[(&str, &str)] = &[
            ("Open File", "Ctrl+O"),
            ("Search Everywhere", "Double Shift"),
            ("Project View", "Alt+1"),
        ];
        let font_action = egui::FontId::proportional(13.0);
        let font_keybind = egui::FontId::proportional(11.0);
        let font_hint = egui::FontId::proportional(12.0);
        let line_height = 26.0;
        let gap = 16.0;
        // 快捷键列表高度 + 间距 + 拖拽提示一行
        let hints_h = line_height * hints.len() as f32;
        let total_h = hints_h + gap + line_height;
        let start_y = center.y - total_h / 2.0;
        for (i, (action, keybind)) in hints.iter().enumerate() {
            let mid_y = start_y + i as f32 * line_height + line_height / 2.0;
            // 操作名（右对齐到中线左侧）
            painter.text(
                egui::pos2(center.x - 8.0, mid_y),
                egui::Align2::RIGHT_CENTER,
                *action,
                font_action.clone(),
                theme::TEXT_MUTED,
            );
            // 快捷键（左对齐到中线右侧，带圆角背景框）
            let keybind_galley = painter.layout_no_wrap(
                keybind.to_string(),
                font_keybind.clone(),
                theme::TEXT_MUTED,
            );
            let kb_pos = egui::pos2(center.x + 8.0, mid_y - keybind_galley.size().y / 2.0);
            let kb_rect = egui::Rect::from_min_size(kb_pos, keybind_galley.size())
                .expand2(egui::vec2(6.0, 2.0));
            painter.rect(
                kb_rect,
                egui::CornerRadius::same(3),
                theme::BG_MEDIUM,
                egui::Stroke::new(1.0, theme::BORDER),
                egui::StrokeKind::Outside,
            );
            painter.galley(kb_pos, keybind_galley, theme::TEXT_MUTED);
        }
        // 底部拖拽提示
        let drop_y = start_y + hints_h + gap + line_height / 2.0;
        painter.text(
            egui::pos2(center.x, drop_y),
            egui::Align2::CENTER_CENTER,
            "Drop files here to open them",
            font_hint,
            theme::TEXT_MUTED,
        );
        ui.allocate_rect(rect, egui::Sense::hover());
    }

    /// 查找当前聚焦的 tab
    fn focused_tab(&mut self) -> Option<&mut EditorTab> {
        // find_active_focused 返回的引用跨越了 else 分支的二次借用，
        // 需要先 is_some() 断开生命周期
        if self.dock_state.find_active_focused().is_some() {
            return self.dock_state.find_active_focused().map(|(_, t)| t);
        }
        self.dock_state
            .main_surface_mut()
            .find_active()
            .map(|(_, t)| t)
    }

    /// 当前活跃 tab 的行号栏宽度（Hex 视图返回 None，自己画地址列）
    fn active_gutter_width(&mut self) -> Option<f32> {
        let tab = self.focused_tab()?;
        let text = match tab.active_view {
            ActiveView::Decompiled => &tab.decompiled,
            ActiveView::Bytecode => &tab.bytecode,
            ActiveView::Hex => return None,
        };
        Some(line_number_width(text.lines().count().max(1)))
    }

    /// 添加新 Tab
    pub fn open_tab(&mut self, tab: EditorTab) {
        self.dock_state.main_surface_mut().push_to_focused_leaf(tab);
    }

    /// 获取当前活跃 Tab 的 active_view（供 StatusBar 显示）
    pub fn focused_view(&mut self) -> Option<ActiveView> {
        Some(self.focused_tab()?.active_view)
    }

    /// 设置当前活跃 Tab 的 active_view（从 StatusBar 切换）
    pub fn set_focused_view(&mut self, view: ActiveView) {
        if let Some(tab) = self.focused_tab() {
            tab.active_view = view;
        }
    }
}
