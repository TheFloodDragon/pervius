//! 左侧面板：文件树
//!
//! @author sky

mod filter;
pub mod tree;

use crate::appearance::theme::flat_button_theme;
use crate::appearance::{codicon, theme};
use crate::task::Task;
use eframe::egui;
use egui_shell::components::FlatButton;
use rust_i18n::t;
use std::collections::HashSet;
use std::sync::Arc;
use tree::TreeNode;

tabookit::class! {
    /// 文件面板状态
    pub struct FilePanel {
        /// 根目录（文件树顶部）
        pub roots: Vec<TreeNode>,
        /// 当前选中项（路径）
        pub selected: Option<String>,
        /// 待打开的文件条目路径（由 App 消费）
        pub pending_open: Option<String>,
        /// 待定位的文件条目路径（由 App 消费，在资源管理器中打开）
        pub pending_reveal: Option<String>,
        /// 需要滚动到选中项
        pub scroll_to_selected: bool,
        /// 速搜过滤文本（键盘直接输入，IntelliJ 风格）
        pub filter: String,
        /// 面板是否拥有隐式键盘焦点（点击面板获得，点击别处失去）
        focused: bool,
        /// 预构建的过滤索引（树变化时重建，Arc 共享给后台线程）
        filter_index: Option<Arc<Vec<tree::FilterEntry>>>,
        /// 当前过滤可见集合（后台线程产出）
        filter_visible: HashSet<String>,
        /// 后台过滤任务
        filter_task: Option<Task<(u64, tree::FilterResult)>>,
        /// 过滤请求计数器（丢弃过期结果）
        filter_gen: u64,
    }

    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            selected: None,
            pending_open: None,
            pending_reveal: None,
            scroll_to_selected: false,
            filter: String::new(),
            focused: false,
            filter_index: None,
            filter_visible: HashSet::new(),
            filter_task: None,
            filter_gen: 0,
        }
    }

    /// 在给定 rect 内渲染（背景由 layout island 绘制）
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        tab_modified: &HashSet<String>,
        jar_modified: &HashSet<String>,
        decompiled_classes: Option<&HashSet<String>>,
    ) {
        let rect = ui.max_rect();
        self.update_focus(ui.ctx(), rect);
        if self.focused {
            self.capture_input(ui.ctx());
        }
        self.poll_filter_result();
        let painter = ui.painter();
        // 面板标题
        let title_h = 32.0;
        let title_rect =
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(rect.width(), title_h));
        painter.text(
            egui::pos2(title_rect.left() + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &t!("explorer.title"),
            egui::FontId::proportional(11.0),
            theme::TEXT_SECONDARY,
        );
        // 标题栏右侧按钮
        self.render_title_buttons(ui, title_rect);
        // 内容区（左 2px、右 8px padding，右侧留空避免文字贴进 scrollbar）
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, title_rect.bottom()),
            egui::pos2(rect.right() - 8.0, rect.bottom()),
        );
        let mut body_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body_rect)
                .id(egui::Id::new("explorer_body")),
        );
        self.render_tree(&mut body_ui, tab_modified, jar_modified, decompiled_classes);
        // 过滤条浮层
        self.render_filter_bar(ui, rect);
    }

    fn render_tree(
        &mut self,
        ui: &mut egui::Ui,
        tab_modified: &HashSet<String>,
        jar_modified: &HashSet<String>,
        decompiled_classes: Option<&HashSet<String>>,
    ) {
        let filtering = !self.filter.is_empty();
        let mut ctx_reveal = None;
        let scroll = self.scroll_to_selected;
        let opened = tree::render_tree(
            ui,
            &mut self.roots,
            &self.selected,
            &self.filter_visible,
            &mut ctx_reveal,
            scroll,
            tab_modified,
            jar_modified,
            decompiled_classes,
        );
        self.scroll_to_selected = false;
        if let Some(path) = opened {
            self.selected = Some(path.clone());
            self.pending_open = Some(path.clone());
            if filtering {
                tree::reveal(&mut self.roots, &path);
                self.scroll_to_selected = true;
            }
            self.clear_filter();
        }
        if ctx_reveal.is_some() {
            self.pending_reveal = ctx_reveal;
        }
    }

    /// 标题栏右侧展开/折叠按钮
    fn render_title_buttons(&mut self, ui: &mut egui::Ui, title_rect: egui::Rect) {
        if self.roots.is_empty() {
            return;
        }
        let fbt = flat_button_theme(theme::TEXT_SECONDARY);
        let btn_size = egui::vec2(22.0, 22.0);
        let mid_y = title_rect.center().y;
        let icon_family = codicon::family();
        // 折叠按钮（最右）
        let collapse_x = title_rect.right() - 8.0 - btn_size.x * 0.5;
        let collapse_rect = egui::Rect::from_center_size(egui::pos2(collapse_x, mid_y), btn_size);
        let mut collapse_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(collapse_rect)
                .id_salt("collapse_btn"),
        );
        if collapse_ui
            .add(
                FlatButton::new(codicon::COLLAPSE_ALL, &fbt)
                    .font_size(14.0)
                    .font_family(icon_family.clone())
                    .inactive_color(theme::TEXT_SECONDARY)
                    .min_size(btn_size),
            )
            .on_hover_text(t!("explorer.collapse"))
            .clicked()
        {
            tree::collapse_one_level(&mut self.roots);
        }
        // 展开按钮
        let expand_x = collapse_rect.left() - 2.0 - btn_size.x * 0.5;
        let expand_rect = egui::Rect::from_center_size(egui::pos2(expand_x, mid_y), btn_size);
        let mut expand_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(expand_rect)
                .id_salt("expand_btn"),
        );
        if expand_ui
            .add(
                FlatButton::new(codicon::EXPAND_ALL, &fbt)
                    .font_size(14.0)
                    .font_family(icon_family)
                    .inactive_color(theme::TEXT_SECONDARY)
                    .min_size(btn_size),
            )
            .on_hover_text(t!("explorer.expand"))
            .clicked()
        {
            tree::expand_one_level(&mut self.roots);
        }
    }
}

use crate::app::App;

impl App {
    /// 编辑器聚焦 tab 变化时同步 explorer 选中状态
    pub(crate) fn sync_explorer_selection(&mut self) {
        if let Some(path) = self.layout.editor.focused_entry_path() {
            if self.layout.file_panel.selected.as_ref() != Some(&path) {
                tree::reveal(&mut self.layout.file_panel.roots, &path);
                self.layout.file_panel.selected = Some(path);
                self.layout.file_panel.scroll_to_selected = true;
            }
        }
    }

    /// 分别收集 tab 级别（橙色）和 JAR 级别（绿色）已修改条目路径（含父级目录）
    pub(crate) fn split_modified_entries(&self) -> (HashSet<String>, HashSet<String>) {
        let mut tab_set = HashSet::new();
        let mut jar_set = HashSet::new();
        for (_, tab) in self.layout.editor.dock_state.iter_all_tabs() {
            if tab.is_modified {
                if let Some(path) = &tab.entry_path {
                    Self::insert_with_parents(&mut tab_set, path);
                }
            }
        }
        if let Some(jar) = self.workspace.jar() {
            for path in jar.modified_paths() {
                Self::insert_with_parents(&mut jar_set, path);
            }
        }
        (tab_set, jar_set)
    }

    /// 将路径及其所有父级目录加入集合
    fn insert_with_parents(set: &mut HashSet<String>, path: &str) {
        set.insert(path.to_string());
        let mut p = path;
        while let Some(idx) = p.rfind('/') {
            let parent = &p[..idx + 1];
            if !set.insert(parent.to_string()) {
                break;
            }
            p = &p[..idx];
        }
        set.insert(String::new());
    }
}
