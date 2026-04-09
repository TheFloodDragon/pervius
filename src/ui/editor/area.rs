//! 编辑器区域：egui_dock 容器 + tab 管理
//!
//! @author sky

use super::find::FindBar;
use super::render::{self, line_number_width};
use super::style::dock;
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use super::viewer::{EditorTabViewer, TabAction};
use crate::decompiler;
use crate::shell::theme;
use eframe::egui;
use egui_dock::{DockArea, DockState, SurfaceIndex, TabPath};

/// 编辑器区域状态
pub struct EditorArea {
    pub dock_state: DockState<EditorTab>,
    pub find_bar: FindBar,
}

impl EditorArea {
    pub fn new() -> Self {
        Self {
            dock_state: DockState::new(vec![]),
            find_bar: FindBar::new(),
        }
    }

    /// 在给定 UI 区域内渲染
    pub fn render(&mut self, ui: &mut egui::Ui) {
        if self.is_empty() {
            Self::render_placeholder(ui);
            return;
        }
        let rect = ui.max_rect();
        if let Some(gutter_w) = self.active_gutter_width() {
            render::paint_editor_bg(ui, rect, gutter_w);
        }
        let style = dock::build(ui.style());
        let mut viewer = EditorTabViewer {
            action: None,
            find_bar: &mut self.find_bar,
        };
        DockArea::new(&mut self.dock_state)
            .style(style)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_inside(ui, &mut viewer);
        if let Some(action) = viewer.action.take() {
            self.handle_tab_action(action);
        }
        ui.output_mut(|o| {
            if matches!(o.cursor_icon, egui::CursorIcon::Grab) {
                o.cursor_icon = egui::CursorIcon::Default;
            }
        });
    }

    fn is_empty(&self) -> bool {
        self.dock_state.main_surface().num_tabs() == 0
    }

    fn render_placeholder(ui: &mut egui::Ui) {
        use crate::ui::keybindings;
        let rect = ui.max_rect();
        let painter = ui.painter();
        let center = rect.center();
        let hints: &[(&str, String)] = &[
            ("Open File", keybindings::DEFAULT_OPEN_JAR.label()),
            ("Find in Files", "Double Shift".into()),
            ("Project View", keybindings::DEFAULT_TOGGLE_EXPLORER.label()),
        ];
        let font_action = egui::FontId::proportional(13.0);
        let font_keybind = egui::FontId::proportional(11.0);
        let font_hint = egui::FontId::proportional(12.0);
        let line_height = 26.0;
        let gap = 16.0;
        let hints_h = line_height * hints.len() as f32;
        let total_h = hints_h + gap + line_height;
        let start_y = center.y - total_h / 2.0;
        for (i, (action, keybind)) in hints.iter().enumerate() {
            let mid_y = start_y + i as f32 * line_height + line_height / 2.0;
            painter.text(
                egui::pos2(center.x - 8.0, mid_y),
                egui::Align2::RIGHT_CENTER,
                *action,
                font_action.clone(),
                theme::TEXT_MUTED,
            );
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

    fn focused_tab(&mut self) -> Option<&mut EditorTab> {
        if self.dock_state.find_active_focused().is_some() {
            return self.dock_state.find_active_focused().map(|(_, t)| t);
        }
        self.dock_state
            .main_surface_mut()
            .find_active()
            .map(|(_, t)| t)
    }

    fn active_gutter_width(&mut self) -> Option<f32> {
        let tab = self.focused_tab()?;
        let text = match tab.active_view {
            ActiveView::Decompiled => &tab.decompiled,
            ActiveView::Bytecode => &tab.bytecode,
            ActiveView::Hex => return None,
        };
        Some(line_number_width(text.lines().count().max(1)))
    }

    pub fn open_tab(&mut self, tab: EditorTab) {
        self.dock_state.main_surface_mut().push_to_focused_leaf(tab);
    }

    /// 聚焦已打开的 tab（按 entry_path 匹配），返回是否找到
    pub fn focus_tab(&mut self, entry_path: &str) -> bool {
        let found = self
            .dock_state
            .find_tab_from(|tab| tab.entry_path.as_deref() == Some(entry_path));
        if let Some(tab_path) = found {
            let _ = self.dock_state.set_active_tab(tab_path);
            true
        } else {
            false
        }
    }

    pub fn focused_view(&mut self) -> Option<ActiveView> {
        Some(self.focused_tab()?.active_view)
    }

    /// 当前聚焦 tab 是否为 .class 文件
    pub fn focused_is_class(&mut self) -> bool {
        self.focused_tab().is_some_and(|t| t.is_class)
    }

    /// 当前聚焦 tab 的 class 版本信息
    pub fn focused_class_info(&mut self) -> Option<&str> {
        self.focused_tab()?.class_info.as_deref()
    }

    /// 当前聚焦 tab 的条目路径
    pub fn focused_entry_path(&mut self) -> Option<String> {
        self.focused_tab()?.entry_path.clone()
    }

    pub fn set_focused_view(&mut self, view: ActiveView) {
        if let Some(tab) = self.focused_tab() {
            tab.active_view = view;
        }
    }

    /// 循环切换聚焦 tab 的视图（仅 .class 文件）
    pub fn cycle_view(&mut self) {
        if let Some(tab) = self.focused_tab() {
            if !tab.is_class {
                return;
            }
            tab.active_view = tab.active_view.next();
        }
    }

    /// 打开/聚焦编辑器内查找栏
    pub fn open_find(&mut self) {
        self.find_bar.toggle();
    }

    /// 反编译完成后，刷新所有已打开的 .class tab 的源码
    pub fn refresh_class_tabs(&mut self, jar_hash: Option<&str>) {
        let hash = match jar_hash {
            Some(h) => h,
            None => return,
        };
        for (_, tab) in self.dock_state.iter_all_tabs_mut() {
            let entry = match &tab.entry_path {
                Some(p) if p.ends_with(".class") => p.clone(),
                _ => continue,
            };
            if let Some(cached) = decompiler::cached_source(hash, &entry) {
                let lang = if cached.is_kotlin {
                    super::highlight::Language::Kotlin
                } else {
                    super::highlight::Language::Java
                };
                tab.decompiled = cached.source;
                tab.language = lang;
                tab.layouter_decompiled = Box::new(super::highlight::make_layouter(lang));
                if tab.active_view == ActiveView::Hex {
                    tab.active_view = ActiveView::Decompiled;
                }
            }
        }
    }

    /// 关闭当前活跃 tab
    pub fn close_active_tab(&mut self) {
        let path = self.dock_state.focused_leaf().or_else(|| {
            let node = self.dock_state[SurfaceIndex::main()].focused_leaf()?;
            Some(egui_dock::NodePath {
                surface: SurfaceIndex::main(),
                node,
            })
        });
        if let Some(node_path) = path {
            let active = self.dock_state[node_path.surface][node_path.node]
                .get_leaf()
                .map(|leaf| leaf.active);
            if let Some(tab_idx) = active {
                self.dock_state.remove_tab(TabPath::new(
                    node_path.surface,
                    node_path.node,
                    tab_idx,
                ));
            }
        }
    }

    /// 关闭所有 tab
    pub fn close_all_tabs(&mut self) {
        self.dock_state = DockState::new(vec![]);
    }

    fn handle_tab_action(&mut self, action: TabAction) {
        match action {
            TabAction::Close(path) => {
                let found = self.dock_state.find_tab_from(|t| t.entry_path == path);
                if let Some(tab_path) = found {
                    self.dock_state.remove_tab(tab_path);
                }
            }
            TabAction::CloseAll => {
                self.dock_state = DockState::new(vec![]);
            }
            TabAction::CloseOthers(keep_path) => {
                self.dock_state
                    .retain_tabs(|tab| tab.entry_path == keep_path);
            }
            TabAction::CloseToRight(after_path) => {
                let found = self
                    .dock_state
                    .find_tab_from(|t| t.entry_path == after_path);
                if let Some(tab_path) = found {
                    let node = &mut self.dock_state[tab_path.surface][tab_path.node];
                    if let Some(leaf) = node.get_leaf_mut() {
                        leaf.tabs.truncate(tab_path.tab.0 + 1);
                        if leaf.active.0 >= leaf.tabs.len() {
                            leaf.active.0 = leaf.tabs.len().saturating_sub(1);
                        }
                    }
                }
            }
        }
    }
}
