//! 搜索浮动面板：egui::Window 承载，可拖动、可 resize、不阻挡后方交互
//!
//! 双模式搜索（Decompiled / Bytecode），结果列表 + 底部预览编辑器
//!
//! @author sky

use super::category::SearchCategory;
use super::demo;
use super::result::SearchResultGroup;
use super::widget::{self, render_group_header, render_match_row, separator, SearchMode};
use crate::appearance::{codicon, theme};
use crate::ui::widget::{flat_button_theme, FlatButton};
use eframe::egui;
use egui_editor::highlight;
use egui_shell::components::FloatingWindow;
use rust_i18n::t;

/// 预览面板行高
const PREVIEW_LINE_HEIGHT: f32 = 18.0;
/// 预览面板行号栏宽度
const GUTTER_WIDTH: f32 = 40.0;
/// 匹配行高亮背景 #43B3AE 12% 不透明度
const MATCH_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgba_premultiplied(5, 22, 21, 30);
/// 匹配文本背景 #43B3AE 25% 不透明度（叠加在语法着色上标识匹配区间）
const MATCH_TEXT_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(17, 45, 44, 64);

/// 缓存预览面板的逐行 LayoutJob，避免每帧重复 tree-sitter 解析
struct PreviewCache {
    /// (group_index, match_index, is_bytecode)
    key: (usize, usize, bool),
    jobs: Vec<egui::text::LayoutJob>,
}

/// 搜索浮动面板
pub struct SearchDialog {
    window: FloatingWindow,
    query: String,
    prev_query: String,
    category: SearchCategory,
    mode: SearchMode,
    results: Vec<SearchResultGroup>,
    focus_input: bool,
    case_sensitive: bool,
    use_regex: bool,
    selected: Option<(usize, usize)>,
    scroll_to_match: bool,
    preview_cache: Option<PreviewCache>,
}

impl SearchDialog {
    pub fn new() -> Self {
        Self {
            window: FloatingWindow::new("search_window", t!("search.title").to_string())
                .icon(codicon::SEARCH)
                .default_size([700.0, 520.0])
                .min_size([400.0, 300.0]),
            query: String::new(),
            prev_query: String::new(),
            category: SearchCategory::Strings,
            mode: SearchMode::Decompiled,
            results: Vec::new(),
            focus_input: false,
            case_sensitive: false,
            use_regex: false,
            selected: None,
            scroll_to_match: false,
            preview_cache: None,
        }
    }

    pub fn open(&mut self) {
        if self.window.is_open() {
            return;
        }
        self.window.open();
        self.focus_input = true;
        self.query.clear();
        self.prev_query.clear();
        self.category = SearchCategory::Strings;
        self.results = demo::demo_results(self.category);
        self.selected = None;
    }

    pub fn render(&mut self, ctx: &egui::Context, shell_theme: &egui_shell::ShellTheme) {
        // 临时取出 window 避免 &mut self 借用冲突
        let mut window = std::mem::take(&mut self.window);
        // header_right 需要的状态提取到局部变量，避免两个闭包同时借用 self
        let mut mode = self.mode;
        let mut mode_changed = false;
        window.show(
            ctx,
            shell_theme,
            |ui| {
                let fbt = flat_button_theme();
                let flat = |label, active| {
                    FlatButton::new(label, &fbt)
                        .font_size(11.0)
                        .active(active)
                        .inactive_color(theme::TEXT_MUTED)
                        .min_size(egui::vec2(0.0, 22.0))
                };
                let label_bytecode = t!("search.bytecode");
                let label_decompiled = t!("search.decompiled");
                if ui
                    .add(flat(&label_bytecode, mode == SearchMode::Bytecode))
                    .clicked()
                {
                    mode = SearchMode::Bytecode;
                    mode_changed = true;
                }
                ui.add_space(2.0);
                if ui
                    .add(flat(&label_decompiled, mode == SearchMode::Decompiled))
                    .clicked()
                {
                    mode = SearchMode::Decompiled;
                    mode_changed = true;
                }
            },
            |ui| {
                self.render_toolbar(ui);
                separator(ui);
                self.render_categories(ui);
                separator(ui);
                if self.query != self.prev_query {
                    self.prev_query = self.query.clone();
                    self.results = demo::demo_results(self.category);
                    self.selected = None;
                }
                self.render_body(ui);
            },
        );
        if mode_changed {
            self.mode = mode;
            self.scroll_to_match = self.selected.is_some();
        }
        self.window = window;
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        let fbt = flat_button_theme();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(codicon::SEARCH)
                    .font(egui::FontId::new(14.0, codicon::family()))
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(4.0);
            let input_width = (ui.available_width() - 80.0).max(120.0);
            let resp = ui.add_sized(
                egui::vec2(input_width, 24.0),
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(t!("search.hint"))
                    .frame(egui::Frame::NONE)
                    .text_color(theme::TEXT_PRIMARY)
                    .font(egui::FontId::proportional(13.0)),
            );
            if self.focus_input {
                resp.request_focus();
                self.focus_input = false;
            }
            ui.add_space(4.0);
            if ui
                .add(
                    FlatButton::new(codicon::CASE_SENSITIVE, &fbt)
                        .font_family(codicon::family())
                        .font_size(15.0)
                        .active(self.case_sensitive)
                        .inactive_color(theme::TEXT_MUTED)
                        .min_size(egui::vec2(28.0, 24.0)),
                )
                .on_hover_text(t!("search.match_case"))
                .clicked()
            {
                self.case_sensitive = !self.case_sensitive;
            }
            ui.add_space(2.0);
            if ui
                .add(
                    FlatButton::new(codicon::REGEX, &fbt)
                        .font_family(codicon::family())
                        .font_size(15.0)
                        .active(self.use_regex)
                        .inactive_color(theme::TEXT_MUTED)
                        .min_size(egui::vec2(28.0, 24.0)),
                )
                .on_hover_text(t!("search.use_regex"))
                .clicked()
            {
                self.use_regex = !self.use_regex;
            }
            ui.add_space(8.0);
        });
        ui.add_space(6.0);
    }

    fn render_categories(&mut self, ui: &mut egui::Ui) {
        let fbt = flat_button_theme();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            for &cat in SearchCategory::ALL {
                let label = cat.label();
                if ui
                    .add(
                        FlatButton::new(&label, &fbt)
                            .font_size(11.0)
                            .active(self.category == cat)
                            .min_size(egui::vec2(0.0, 22.0)),
                    )
                    .clicked()
                {
                    self.category = cat;
                    self.results = demo::demo_results(self.category);
                    self.selected = None;
                }
                ui.add_space(2.0);
            }
        });
        ui.add_space(4.0);
    }

    fn render_body(&mut self, ui: &mut egui::Ui) {
        if self.results.is_empty() {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(t!("search.no_results"))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            });
            return;
        }
        let avail = ui.available_height();
        let list_h = (avail * 0.42).max(80.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), list_h), |ui| {
            self.render_results(ui);
        });
        separator(ui);
        self.render_preview(ui);
    }

    fn render_results(&mut self, ui: &mut egui::Ui) {
        let mode = self.mode;
        egui::ScrollArea::vertical()
            .id_salt("search_results")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(2.0);
                let mut toggle_idx: Option<usize> = None;
                let mut click: Option<(usize, usize)> = None;
                for (gi, group) in self.results.iter().enumerate() {
                    ui.push_id(gi, |ui| {
                        if render_group_header(ui, group) {
                            toggle_idx = Some(gi);
                        }
                        if group.expanded {
                            for (mi, m) in group.matches.iter().enumerate() {
                                ui.push_id(mi, |ui| {
                                    let sel = self.selected == Some((gi, mi));
                                    if render_match_row(ui, m, sel, mode) {
                                        click = Some((gi, mi));
                                    }
                                });
                            }
                        }
                    });
                }
                if let Some(i) = toggle_idx {
                    self.results[i].expanded = !self.results[i].expanded;
                }
                if let Some(key) = click {
                    if self.selected != Some(key) {
                        self.selected = Some(key);
                        self.scroll_to_match = true;
                    }
                }
                ui.add_space(4.0);
            });
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        let (gi, mi) = tabookit::or!(self.selected, {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(t!("search.select_preview"))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            });
            return;
        });
        let m = tabookit::or!(self.results.get(gi).and_then(|g| g.matches.get(mi)), return);
        let sp = widget::preview_for(m, self.mode);
        let bytecode = self.mode == SearchMode::Bytecode;
        let cache_key = (gi, mi, bytecode);
        // 缓存失效时重新计算语法高亮
        let need_recompute = self
            .preview_cache
            .as_ref()
            .map_or(true, |c| c.key != cache_key);
        if need_recompute {
            // 将 preview 的匹配区间映射到 source_lines[match_line] 内的字节偏移
            let line_ranges = if sp.match_line < sp.source_lines.len() {
                let src_line = &sp.source_lines[sp.match_line];
                let offset = src_line.find(&sp.preview).unwrap_or(0);
                sp.highlight_ranges
                    .iter()
                    .map(|&(s, e)| (s + offset, e + offset))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let lang = if bytecode {
                highlight::Language::Bytecode
            } else {
                highlight::Language::Java
            };
            let jobs = highlight::highlight_per_line(
                &sp.source_lines,
                lang,
                egui::FontId::monospace(11.0),
                sp.match_line,
                &line_ranges,
                MATCH_TEXT_BG,
                &theme::editor_theme().syntax,
            );
            self.preview_cache = Some(PreviewCache {
                key: cache_key,
                jobs,
            });
        }
        let jobs = &self.preview_cache.as_ref().expect("just computed").jobs;
        let scroll_to = self.scroll_to_match;
        self.scroll_to_match = false;
        let mono = egui::FontId::monospace(11.0);
        let avail_w = ui.available_width();
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("search_preview")
            .auto_shrink([false, false]);
        if scroll_to {
            let target_y = sp.match_line as f32 * PREVIEW_LINE_HEIGHT;
            scroll = scroll.vertical_scroll_offset(
                target_y.max(PREVIEW_LINE_HEIGHT * 3.0) - PREVIEW_LINE_HEIGHT * 3.0,
            );
        }
        scroll.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, _line) in sp.source_lines.iter().enumerate() {
                let is_match = i == sp.match_line;
                ui.push_id(i, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(avail_w, PREVIEW_LINE_HEIGHT),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();
                    if is_match {
                        painter.rect_filled(rect, 0.0, MATCH_HIGHLIGHT);
                    }
                    let mid_y = rect.center().y;
                    painter.text(
                        egui::pos2(rect.left() + GUTTER_WIDTH - 4.0, mid_y),
                        egui::Align2::RIGHT_CENTER,
                        &format!("{}", i + 1),
                        mono.clone(),
                        theme::TEXT_MUTED,
                    );
                    let text_x = rect.left() + GUTTER_WIDTH + 8.0;
                    if let Some(job) = jobs.get(i) {
                        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job.clone()));
                        painter.galley(
                            egui::pos2(text_x, mid_y - galley.size().y / 2.0),
                            galley,
                            egui::Color32::PLACEHOLDER,
                        );
                    }
                });
            }
        });
    }
}
