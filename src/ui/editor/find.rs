//! 编辑器内查找栏：Ctrl+F 触发，右上角浮动 island
//!
//! @author sky

use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use crate::shell::{codicon, theme};
use crate::ui::widget::FlatButton;
use eframe::egui;
use rust_i18n::t;
use std::hash::{Hash, Hasher};

/// island 到编辑区边缘的间距
const MARGIN: f32 = 8.0;
/// island 高度
const BAR_H: f32 = 30.0;

/// 查找匹配项（字节偏移）
#[derive(Clone)]
pub struct FindMatch {
    pub start: usize,
    pub end: usize,
}

/// 编辑器内查找栏
pub struct FindBar {
    pub open: bool,
    query: String,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
    matches: Vec<FindMatch>,
    current: usize,
    focus_input: bool,
    /// 缓存指纹，避免重复搜索
    last_key: u64,
}

impl FindBar {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            matches: Vec::new(),
            current: 0,
            focus_input: false,
            last_key: 0,
        }
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus_input = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.matches.clear();
        self.current = 0;
        self.last_key = 0;
    }

    /// 更新搜索结果（在渲染内容前调用，提供高亮数据）
    pub fn update(&mut self, tab: &EditorTab) {
        let text = text_for_view(tab);
        self.update_search(text);
    }

    /// 返回当前匹配列表（克隆，避免借用冲突）
    pub fn highlight_info(&self) -> (Vec<FindMatch>, Option<usize>) {
        if self.matches.is_empty() {
            return (Vec::new(), None);
        }
        (self.matches.clone(), Some(self.current))
    }

    /// 渲染浮动 island（在内容渲染之后调用，固定 rect overlay）
    pub fn render_overlay(&mut self, ui: &mut egui::Ui, content_rect: egui::Rect) {
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(content_rect.left() + MARGIN, content_rect.top() + MARGIN),
            egui::vec2(content_rect.width() - MARGIN * 2.0, BAR_H),
        );
        // 阻断下方内容的点击
        ui.interact(
            bar_rect,
            ui.id().with("find_blocker"),
            egui::Sense::click_and_drag(),
        );
        // Island 背景 + 边框
        let painter = ui.painter();
        painter.rect_filled(bar_rect, 6.0, theme::BG_DARK);
        painter.rect_stroke(
            bar_rect,
            6.0,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Inside,
        );
        // 内容
        let inner = bar_rect.shrink2(egui::vec2(8.0, 0.0));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .id_salt("find_bar_overlay"),
        );
        child.set_clip_rect(bar_rect);
        self.render_contents(&mut child);
    }

    fn render_contents(&mut self, ui: &mut egui::Ui) {
        // 右到左布局：按钮固定居右，输入框占剩余空间
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // ✕ 关闭
            if ui
                .add(icon_btn(codicon::CLOSE))
                .on_hover_text(t!("find.close"))
                .clicked()
            {
                self.close();
            }
            ui.add_space(2.0);
            // ↓ 下一个
            if ui
                .add(icon_btn(codicon::CHEVRON_DOWN))
                .on_hover_text(t!("find.next"))
                .clicked()
            {
                self.next_match();
                self.focus_input = true;
            }
            // ↑ 上一个
            if ui
                .add(icon_btn(codicon::CHEVRON_UP))
                .on_hover_text(t!("find.prev"))
                .clicked()
            {
                self.prev_match();
                self.focus_input = true;
            }
            ui.add_space(2.0);
            // 结果计数（始终渲染以保持 widget 数量稳定，避免布局偏移触发 egui ID 冲突警告）
            let (count_text, count_color) = if self.query.is_empty() {
                (String::new(), theme::TEXT_MUTED)
            } else if self.matches.is_empty() {
                (t!("find.no_results").to_string(), theme::ACCENT_RED)
            } else {
                (
                    t!(
                        "find.result_count",
                        current = self.current + 1,
                        total = self.matches.len()
                    )
                    .to_string(),
                    theme::TEXT_MUTED,
                )
            };
            ui.label(
                egui::RichText::new(count_text)
                    .size(12.0)
                    .color(count_color),
            );
            ui.add_space(4.0);
            // .* Regex
            if ui
                .add(icon_toggle(codicon::REGEX, self.regex))
                .on_hover_text(t!("find.use_regex"))
                .clicked()
            {
                self.regex = !self.regex;
                self.invalidate();
                self.focus_input = true;
            }
            // W Whole Word
            if ui
                .add(icon_toggle(codicon::WHOLE_WORD, self.whole_word))
                .on_hover_text(t!("find.match_word"))
                .clicked()
            {
                self.whole_word = !self.whole_word;
                self.invalidate();
                self.focus_input = true;
            }
            // Cc Case Sensitive
            if ui
                .add(icon_toggle(codicon::CASE_SENSITIVE, self.case_sensitive))
                .on_hover_text(t!("find.match_case"))
                .clicked()
            {
                self.case_sensitive = !self.case_sensitive;
                self.invalidate();
                self.focus_input = true;
            }
            ui.add_space(4.0);
            // 搜索图标 + 输入框（占剩余全部宽度）
            let remaining = ui.available_width();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(codicon::SEARCH)
                        .font(egui::FontId::new(14.0, codicon::family()))
                        .color(theme::TEXT_MUTED),
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .id(ui.id().with("find_input"))
                        .hint_text(t!("find.hint"))
                        .frame(egui::Frame::NONE)
                        .text_color(theme::TEXT_PRIMARY)
                        .font(egui::FontId::proportional(13.0))
                        .desired_width(remaining - 22.0),
                );
                if self.focus_input {
                    resp.request_focus();
                    self.focus_input = false;
                }
                if resp.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.close();
                        return;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if ui.input(|i| i.modifiers.shift) {
                            self.prev_match();
                        } else {
                            self.next_match();
                        }
                        self.focus_input = true;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::F3)) {
                    if ui.input(|i| i.modifiers.shift) {
                        self.prev_match();
                    } else {
                        self.next_match();
                    }
                }
            });
        });
    }

    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = (self.current + 1) % self.matches.len();
        }
    }

    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current = if self.current == 0 {
                self.matches.len() - 1
            } else {
                self.current - 1
            };
        }
    }

    fn invalidate(&mut self) {
        self.last_key = 0;
    }

    fn update_search(&mut self, text: &str) {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.query.hash(&mut h);
        self.case_sensitive.hash(&mut h);
        self.whole_word.hash(&mut h);
        self.regex.hash(&mut h);
        (text.as_ptr() as usize).hash(&mut h);
        text.len().hash(&mut h);
        let key = h.finish();
        if key == self.last_key {
            return;
        }
        self.last_key = key;
        self.matches = find_all(text, &self.query, self.case_sensitive, self.whole_word);
        if self.matches.is_empty() {
            self.current = 0;
        } else if self.current >= self.matches.len() {
            self.current = 0;
        }
    }
}

// -- 按钮工厂 --

/// Codicon 图标 toggle 按钮（带 active 状态）
fn icon_toggle(icon: &str, active: bool) -> FlatButton<'_> {
    FlatButton::new(icon)
        .font_family(codicon::family())
        .font_size(15.0)
        .active(active)
        .inactive_color(theme::TEXT_MUTED)
        .min_size(egui::vec2(24.0, 22.0))
}

/// Codicon 图标普通按钮
fn icon_btn(icon: &str) -> FlatButton<'_> {
    FlatButton::new(icon)
        .font_family(codicon::family())
        .font_size(14.0)
        .inactive_color(theme::TEXT_MUTED)
        .min_size(egui::vec2(22.0, 22.0))
}

// -- 工具函数 --

fn text_for_view(tab: &EditorTab) -> &str {
    match tab.active_view {
        ActiveView::Decompiled => &tab.decompiled,
        ActiveView::Bytecode => tab.selected_bytecode_text(),
        ActiveView::Hex => "",
    }
}

// -- 搜索引擎 --

fn find_all(text: &str, query: &str, case_sensitive: bool, whole_word: bool) -> Vec<FindMatch> {
    if query.is_empty() {
        return Vec::new();
    }
    if case_sensitive {
        find_plain(text, query, whole_word)
    } else {
        find_case_insensitive(text, query, whole_word)
    }
}

fn find_plain(text: &str, query: &str, whole_word: bool) -> Vec<FindMatch> {
    let mut results = Vec::new();
    let mut start = 0;
    while let Some(pos) = text[start..].find(query) {
        let abs = start + pos;
        let end = abs + query.len();
        if !whole_word || is_word_boundary(text, abs, end) {
            results.push(FindMatch { start: abs, end });
        }
        // 按字符步进，避免落在多字节 UTF-8 字符中间
        let step = text[abs..].chars().next().map_or(1, |c| c.len_utf8());
        start = abs + step;
    }
    results
}

fn find_case_insensitive(text: &str, query: &str, whole_word: bool) -> Vec<FindMatch> {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut results = Vec::new();
    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&query_lower) {
        let abs = start + pos;
        let end = abs + query_lower.len();
        if !whole_word || is_word_boundary(text, abs, end) {
            results.push(FindMatch { start: abs, end });
        }
        let step = text_lower[abs..].chars().next().map_or(1, |c| c.len_utf8());
        start = abs + step;
    }
    results
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = if start == 0 {
        true
    } else {
        text[..start]
            .chars()
            .next_back()
            .map_or(true, |c| !c.is_alphanumeric() && c != '_')
    };
    let after = if end >= text.len() {
        true
    } else {
        text[end..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != '_')
    };
    before && after
}
