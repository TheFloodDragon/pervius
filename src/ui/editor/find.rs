//! 编辑器内查找栏：Ctrl+F 触发，支持大小写敏感、全词匹配
//!
//! @author sky

use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use crate::shell::{codicon, theme};
use crate::ui::widget::FlatButton;
use eframe::egui;
use std::hash::{Hash, Hasher};

/// 查找栏高度
const BAR_HEIGHT: f32 = 30.0;

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

    /// 渲染查找栏并更新搜索结果
    pub fn render(&mut self, ui: &mut egui::Ui, tab: &EditorTab) {
        let text = Self::text_for_view(tab);
        self.update_search(text);
        self.render_bar(ui);
    }

    /// 返回当前匹配列表（克隆，避免借用冲突）
    pub fn highlight_info(&self) -> (Vec<FindMatch>, Option<usize>) {
        if self.matches.is_empty() {
            return (Vec::new(), None);
        }
        (self.matches.clone(), Some(self.current))
    }

    /// 获取当前视图的搜索文本
    fn text_for_view(tab: &EditorTab) -> &str {
        match tab.active_view {
            ActiveView::Decompiled => &tab.decompiled,
            ActiveView::Bytecode => &tab.bytecode,
            ActiveView::Hex => "",
        }
    }

    fn render_bar(&mut self, ui: &mut egui::Ui) {
        let (_, bar_rect) = ui.allocate_space(egui::vec2(ui.available_width(), BAR_HEIGHT));
        let painter = ui.painter();
        // 背景 + 底部分隔线
        painter.rect_filled(bar_rect, 0.0, theme::BG_DARK);
        painter.line_segment(
            [bar_rect.left_bottom(), bar_rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        let inner = bar_rect.shrink2(egui::vec2(8.0, 0.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.set_clip_rect(bar_rect);
        child.horizontal_centered(|ui| {
            self.render_contents(ui);
        });
    }

    fn render_contents(&mut self, ui: &mut egui::Ui) {
        // 搜索图标
        ui.label(
            egui::RichText::new(codicon::SEARCH)
                .font(egui::FontId::new(14.0, codicon::family()))
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(4.0);
        // 输入框
        let input_w = (ui.available_width() - 220.0).max(80.0);
        let resp = ui.add_sized(
            egui::vec2(input_w, 22.0),
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Find...")
                .frame(egui::Frame::NONE)
                .text_color(theme::TEXT_PRIMARY)
                .font(egui::FontId::proportional(13.0)),
        );
        if self.focus_input {
            resp.request_focus();
            self.focus_input = false;
        }
        // 失焦处理：Enter 导航 / Esc 关闭
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
        // F3 导航（全局）
        if ui.input(|i| i.key_pressed(egui::Key::F3)) {
            if ui.input(|i| i.modifiers.shift) {
                self.prev_match();
            } else {
                self.next_match();
            }
        }
        ui.add_space(4.0);
        // Cc — Case Sensitive
        if ui
            .add(
                FlatButton::new(codicon::CASE_SENSITIVE)
                    .font_family(codicon::family())
                    .font_size(15.0)
                    .active(self.case_sensitive)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(26.0, 22.0)),
            )
            .on_hover_text("Match case")
            .clicked()
        {
            self.case_sensitive = !self.case_sensitive;
            self.invalidate();
        }
        ui.add_space(1.0);
        // W — Whole Word
        if ui
            .add(
                FlatButton::new(codicon::WHOLE_WORD)
                    .font_family(codicon::family())
                    .font_size(15.0)
                    .active(self.whole_word)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(26.0, 22.0)),
            )
            .on_hover_text("Match whole word")
            .clicked()
        {
            self.whole_word = !self.whole_word;
            self.invalidate();
        }
        ui.add_space(1.0);
        // .* — Regex
        if ui
            .add(
                FlatButton::new(codicon::REGEX)
                    .font_family(codicon::family())
                    .font_size(15.0)
                    .active(self.regex)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(26.0, 22.0)),
            )
            .on_hover_text("Use regex")
            .clicked()
        {
            self.regex = !self.regex;
            self.invalidate();
        }
        ui.add_space(8.0);
        // 结果计数
        if !self.query.is_empty() {
            let text = if self.matches.is_empty() {
                "0 results".to_string()
            } else {
                format!("{} of {}", self.current + 1, self.matches.len())
            };
            let color = if self.matches.is_empty() {
                theme::ACCENT_RED
            } else {
                theme::TEXT_MUTED
            };
            ui.label(egui::RichText::new(text).size(12.0).color(color));
        }
        ui.add_space(6.0);
        // ↑ 上一个
        if ui
            .add(
                FlatButton::new(codicon::CHEVRON_UP)
                    .font_family(codicon::family())
                    .font_size(14.0)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(22.0, 22.0)),
            )
            .on_hover_text("Previous match (Shift+Enter)")
            .clicked()
        {
            self.prev_match();
        }
        ui.add_space(1.0);
        // ↓ 下一个
        if ui
            .add(
                FlatButton::new(codicon::CHEVRON_DOWN)
                    .font_family(codicon::family())
                    .font_size(14.0)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(22.0, 22.0)),
            )
            .on_hover_text("Next match (Enter)")
            .clicked()
        {
            self.next_match();
        }
        ui.add_space(4.0);
        // ✕ 关闭
        if ui
            .add(
                FlatButton::new(codicon::CLOSE)
                    .font_family(codicon::family())
                    .font_size(14.0)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(22.0, 22.0)),
            )
            .on_hover_text("Close (Esc)")
            .clicked()
        {
            self.close();
        }
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
        start = abs + 1;
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
        start = abs + 1;
    }
    results
}

/// 判断 [start, end) 范围是否构成完整单词
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

// -- 高亮绘制 --

/// 在 TextEdit 输出上绘制查找匹配高亮
pub fn paint_highlights(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    text: &str,
    matches: &[FindMatch],
    current: Option<usize>,
) {
    if matches.is_empty() {
        return;
    }
    let painter = ui.painter();
    let galley = &output.galley;
    let pos = output.galley_pos;
    let clip = output.text_clip_rect;
    for (i, m) in matches.iter().enumerate() {
        // 字节偏移 → 字符偏移（ASCII 快速路径）
        let cs = byte_to_char(text, m.start);
        let ce = byte_to_char(text, m.end);
        let r0 = galley.pos_from_cursor(egui::text::CCursor::new(cs));
        let r1 = galley.pos_from_cursor(egui::text::CCursor::new(ce));
        // 构建屏幕坐标 rect
        let rect = egui::Rect::from_min_max(
            egui::pos2(pos.x + r0.min.x, pos.y + r0.min.y),
            egui::pos2(pos.x + r1.min.x, pos.y + r1.max.y),
        );
        if !rect.intersects(clip) {
            continue;
        }
        let is_current = current == Some(i);
        if is_current {
            painter.rect_filled(rect, 2.0, theme::verdigris_alpha(60));
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, theme::VERDIGRIS),
                egui::StrokeKind::Outside,
            );
        } else {
            painter.rect_filled(rect, 2.0, theme::verdigris_alpha(25));
        }
    }
}

/// 字节偏移 → 字符偏移
fn byte_to_char(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].chars().count()
}
