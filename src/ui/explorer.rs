//! 左侧面板：文件树
//!
//! @author sky

pub mod tree;

use crate::shell::{codicon, theme};
use crate::ui::widget::FlatButton;
use eframe::egui;
use rust_i18n::t;
use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use tree::TreeNode;

/// 文件面板状态
pub struct FilePanel {
    pub roots: Vec<TreeNode>,
    pub selected: Option<String>,
    /// 待打开的文件条目路径（由 Layout 消费）
    pub pending_open: Option<String>,
    /// 待定位的文件条目路径（由 Layout 消费，在资源管理器中打开）
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
    /// 后台过滤结果接收端
    filter_rx: Option<mpsc::Receiver<(u64, tree::FilterResult)>>,
    /// 过滤请求计数器（丢弃过期结果）
    filter_gen: u64,
}

impl FilePanel {
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
            filter_rx: None,
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

    /// 更新面板隐式焦点：点击面板内获得，点击面板外或有 widget 聚焦时失去
    fn update_focus(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        // 有文本控件聚焦时（如编辑器 TextEdit、查找栏）让出焦点
        if ctx.memory(|m| m.focused().is_some()) {
            if self.focused {
                self.focused = false;
                self.clear_filter();
            }
            return;
        }
        // 检测主键点击位置
        if ctx.input(|i| i.pointer.primary_clicked()) {
            let inside = ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| rect.contains(p));
            if inside {
                self.focused = true;
            } else {
                self.focused = false;
                self.clear_filter();
            }
        }
    }

    /// 捕获键盘输入用于速搜过滤（仅在面板拥有焦点时调用）
    fn capture_input(&mut self, ctx: &egui::Context) {
        if self.roots.is_empty() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut changed = false;
        for event in &events {
            match event {
                egui::Event::Text(text) => {
                    // 忽略组合键产生的文本事件（Alt+1、Ctrl+O 等）
                    let has_modifier = ctx.input(|i| {
                        let m = i.modifiers;
                        m.alt || m.ctrl || m.command
                    });
                    if !has_modifier {
                        self.filter.push_str(text);
                        changed = true;
                    }
                }
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.any() && !self.filter.is_empty() => {
                    self.filter.pop();
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if !self.filter.is_empty() => {
                    self.clear_filter();
                }
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } if !self.filter.is_empty() => {
                    if let Some(path) = self.selected.clone() {
                        tree::reveal(&mut self.roots, &path);
                        self.pending_open = Some(path);
                        self.scroll_to_selected = true;
                    }
                    self.clear_filter();
                }
                _ => {}
            }
        }
        if changed {
            self.dispatch_filter();
        }
    }

    /// 发起后台过滤计算
    fn dispatch_filter(&mut self) {
        self.filter_gen += 1;
        if self.filter.is_empty() {
            self.filter_visible.clear();
            return;
        }
        // 首次过滤时构建索引
        if self.filter_index.is_none() {
            self.filter_index = Some(Arc::new(tree::build_filter_index(&self.roots)));
        }
        let lower = self.filter.to_ascii_lowercase();
        let index = Arc::clone(self.filter_index.as_ref().unwrap());
        let seq = self.filter_gen;
        let (tx, rx) = mpsc::channel();
        self.filter_rx = Some(rx);
        std::thread::spawn(move || {
            let result = tree::compute_filter(&index, &lower);
            let _ = tx.send((seq, result));
        });
    }

    /// 拉取后台过滤结果（每帧调用，非阻塞）
    fn poll_filter_result(&mut self) {
        let rx = match &self.filter_rx {
            Some(rx) => rx,
            None => return,
        };
        let (seq, result) = match rx.try_recv() {
            Ok(v) => v,
            Err(_) => return,
        };
        self.filter_rx = None;
        // 丢弃过期结果
        if seq != self.filter_gen {
            return;
        }
        self.filter_visible = result.visible;
        if result.first_match.is_some() {
            self.selected = result.first_match;
            self.scroll_to_selected = true;
        }
    }

    /// 清除过滤状态
    fn clear_filter(&mut self) {
        self.filter.clear();
        self.filter_visible.clear();
        self.filter_gen += 1;
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
                FlatButton::new(codicon::COLLAPSE_ALL)
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
                FlatButton::new(codicon::EXPAND_ALL)
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

    /// 在面板底部绘制过滤条浮层
    fn render_filter_bar(&self, ui: &egui::Ui, rect: egui::Rect) {
        if self.filter.is_empty() {
            return;
        }
        let painter = ui.painter();
        let bar_h = 24.0;
        let m = 6.0;
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + m, rect.bottom() - bar_h - m),
            egui::pos2(rect.right() - m, rect.bottom() - m),
        );
        painter.rect_filled(bar_rect, 4.0, theme::BG_MEDIUM);
        painter.rect_stroke(
            bar_rect,
            4.0,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Middle,
        );
        // 搜索图标
        painter.text(
            egui::pos2(bar_rect.left() + 10.0, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            codicon::SEARCH,
            egui::FontId::new(11.0, codicon::family()),
            theme::TEXT_MUTED,
        );
        // 过滤文本
        painter.text(
            egui::pos2(bar_rect.left() + 26.0, bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &self.filter,
            egui::FontId::proportional(12.0),
            theme::TEXT_PRIMARY,
        );
    }
}
