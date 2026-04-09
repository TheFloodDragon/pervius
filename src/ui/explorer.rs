//! 左侧面板：文件树
//!
//! @author sky

pub mod tree;

use crate::shell::{codicon, theme};
use eframe::egui;
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
        }
    }

    /// 在给定 rect 内渲染（背景由 layout island 绘制）
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.capture_input(ui.ctx());
        let rect = ui.max_rect();
        let painter = ui.painter();
        // 面板标题
        let title_h = 32.0;
        let title_rect =
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(rect.width(), title_h));
        painter.text(
            egui::pos2(title_rect.left() + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "EXPLORER",
            egui::FontId::proportional(11.0),
            theme::TEXT_SECONDARY,
        );
        // 内容区（左右 2px padding）
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 2.0, title_rect.bottom()),
            egui::pos2(rect.right() - 2.0, rect.bottom()),
        );
        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
        self.render_tree(&mut body_ui);
        // 过滤条浮层
        self.render_filter_bar(ui, rect);
    }

    /// 捕获键盘输入用于速搜过滤
    fn capture_input(&mut self, ctx: &egui::Context) {
        // 有文本控件聚焦时不捕获（如搜索对话框、编辑器）
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        if self.roots.is_empty() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut changed = false;
        for event in &events {
            match event {
                egui::Event::Text(text) => {
                    self.filter.push_str(text);
                    changed = true;
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
                    self.filter.clear();
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
                    self.filter.clear();
                }
                _ => {}
            }
        }
        // 过滤变化时自动选中第一个匹配文件
        if changed && !self.filter.is_empty() {
            let lower = self.filter.to_ascii_lowercase();
            self.selected = tree::first_match(&self.roots, &lower);
        }
    }

    fn render_tree(&mut self, ui: &mut egui::Ui) {
        let filtering = !self.filter.is_empty();
        let filter = self.filter.to_ascii_lowercase();
        let mut ctx_reveal = None;
        let scroll = self.scroll_to_selected;
        let mut opened = None;
        egui::ScrollArea::vertical()
            .id_salt("file_tree")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space(4.0);
                opened = tree::render_tree(
                    ui,
                    &mut self.roots,
                    0,
                    &self.selected,
                    &filter,
                    &mut ctx_reveal,
                    scroll,
                );
                ui.add_space(4.0);
            });
        self.scroll_to_selected = false;
        if let Some(path) = opened {
            self.selected = Some(path.clone());
            self.pending_open = Some(path.clone());
            // 过滤模式下点击文件：展开路径，否则清除过滤后节点会隐藏
            if filtering {
                tree::reveal(&mut self.roots, &path);
                self.scroll_to_selected = true;
            }
            self.filter.clear();
        }
        if ctx_reveal.is_some() {
            self.pending_reveal = ctx_reveal;
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
