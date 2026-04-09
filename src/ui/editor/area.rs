//! 编辑器区域：egui_dock 容器 + tab 管理
//!
//! @author sky

use super::dock_style;
use super::render::{self, line_number_width};
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use super::viewer::EditorTabViewer;
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
        let rect = ui.max_rect();
        // 在 DockArea（ScrollArea）外画全高 gutter 背景
        if let Some(gutter_w) = self.active_gutter_width() {
            render::paint_editor_bg(ui, rect, gutter_w);
        }
        let style = dock_style::build(ui.style());
        DockArea::new(&mut self.dock_state)
            .style(style)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_inside(ui, &mut EditorTabViewer);
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

    /// 当前活跃 tab 的行号栏宽度（Hex 视图无行号返回 None）
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
