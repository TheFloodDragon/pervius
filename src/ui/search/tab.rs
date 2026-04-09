//! 搜索浮动面板：egui::Window 承载，可拖动、可 resize、不阻挡后方交互
//!
//! 双模式搜索（Decompiled / Bytecode），结果列表 + 底部预览编辑器
//!
//! @author sky

use super::category::SearchCategory;
use super::result::{SearchMatch, SearchResultGroup, SourcePreview};
use crate::shell::{codicon, theme};
use eframe::egui;

const ROW_HEIGHT: f32 = 24.0;
const GROUP_HEADER_HEIGHT: f32 = 28.0;
/// 自定义标题栏高度（加厚以便拖拽）
const HEADER_HEIGHT: f32 = 32.0;
/// 预览面板行高
const PREVIEW_LINE_HEIGHT: f32 = 18.0;
/// 预览面板行号栏宽度
const GUTTER_WIDTH: f32 = 40.0;
/// 匹配行高亮背景 #43B3AE 12% 不透明度
const MATCH_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgba_premultiplied(5, 22, 21, 30);

/// 预览视图模式
#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Decompiled,
    Bytecode,
}

/// 搜索浮动面板
pub struct SearchDialog {
    open: bool,
    pinned: bool,
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
}

impl SearchDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            pinned: false,
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
        }
    }

    pub fn open(&mut self) {
        if self.open {
            return;
        }
        self.open = true;
        self.focus_input = true;
        self.query.clear();
        self.prev_query.clear();
        self.category = SearchCategory::Strings;
        self.results = demo_results(self.category);
        self.selected = None;
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        if !self.pinned && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
            return;
        }
        let resp = egui::Window::new("search_window_title")
            .id("search_window".into())
            .title_bar(false)
            .movable(true)
            .resizable(true)
            .collapsible(false)
            .default_size(egui::vec2(700.0, 520.0))
            .min_size(egui::vec2(400.0, 300.0))
            .frame(egui::Frame {
                fill: theme::BG_MEDIUM,
                corner_radius: egui::CornerRadius::same(8),
                stroke: egui::Stroke::new(1.0, theme::BORDER_LIGHT),
                inner_margin: egui::Margin::same(0),
                shadow: egui::Shadow {
                    spread: 2,
                    blur: 20,
                    offset: [0, 4],
                    color: egui::Color32::from_black_alpha(80),
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                let style = ui.style_mut();
                style.visuals.widgets.noninteractive.fg_stroke =
                    egui::Stroke::new(1.0, theme::TEXT_PRIMARY);
                style.visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
                style.visuals.widgets.active.bg_fill = theme::BG_LIGHT;
                style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                self.render_header(ui);
                separator(ui);
                self.render_toolbar(ui);
                separator(ui);
                self.render_categories(ui);
                separator(ui);
                if self.query != self.prev_query {
                    self.prev_query = self.query.clone();
                    self.results = demo_results(self.category);
                    self.selected = None;
                }
                self.render_body(ui);
            });
        if !self.pinned {
            if let Some(inner) = &resp {
                let wr = inner.response.rect.expand(4.0);
                let clicked_outside = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| !wr.contains(p))
                });
                if clicked_outside {
                    self.open = false;
                }
            }
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.set_height(HEADER_HEIGHT);
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(codicon::SEARCH)
                    .font(egui::FontId::new(14.0, codicon::family()))
                    .color(theme::VERDIGRIS),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Search")
                    .font(egui::FontId::proportional(13.0))
                    .color(theme::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                // Pin 按钮
                let pin_color = if self.pinned {
                    theme::VERDIGRIS
                } else {
                    theme::TEXT_MUTED
                };
                let pin_fill = if self.pinned {
                    theme::BG_LIGHT
                } else {
                    egui::Color32::TRANSPARENT
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(codicon::PIN)
                                .font(egui::FontId::new(14.0, codicon::family()))
                                .color(pin_color),
                        )
                        .fill(pin_fill)
                        .corner_radius(4)
                        .min_size(egui::vec2(26.0, 24.0)),
                    )
                    .on_hover_text(if self.pinned { "Unpin" } else { "Pin" })
                    .clicked()
                {
                    self.pinned = !self.pinned;
                }
                ui.add_space(8.0);
                // 模式切换：Bytecode | Decompiled（right-to-left 顺序）
                if mode_button(ui, "Bytecode", self.mode == SearchMode::Bytecode) {
                    self.mode = SearchMode::Bytecode;
                    self.scroll_to_match = self.selected.is_some();
                }
                ui.add_space(2.0);
                if mode_button(ui, "Decompiled", self.mode == SearchMode::Decompiled) {
                    self.mode = SearchMode::Decompiled;
                    self.scroll_to_match = self.selected.is_some();
                }
            });
        });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
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
                    .hint_text("Search...")
                    .frame(egui::Frame::NONE)
                    .text_color(theme::TEXT_PRIMARY)
                    .font(egui::FontId::proportional(13.0)),
            );
            if self.focus_input {
                resp.request_focus();
                self.focus_input = false;
            }
            ui.add_space(4.0);
            if toggle_icon(
                ui,
                codicon::CASE_SENSITIVE,
                self.case_sensitive,
                "Match case",
            ) {
                self.case_sensitive = !self.case_sensitive;
            }
            ui.add_space(2.0);
            if toggle_icon(ui, codicon::REGEX, self.use_regex, "Use regex") {
                self.use_regex = !self.use_regex;
            }
            ui.add_space(8.0);
        });
        ui.add_space(6.0);
    }

    fn render_categories(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            for &cat in SearchCategory::ALL {
                let active = self.category == cat;
                let color = if active {
                    theme::VERDIGRIS
                } else {
                    theme::TEXT_SECONDARY
                };
                let fill = if active {
                    theme::BG_HOVER
                } else {
                    egui::Color32::TRANSPARENT
                };
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(cat.label()).size(11.0).color(color))
                            .fill(fill)
                            .corner_radius(3)
                            .min_size(egui::vec2(0.0, 22.0)),
                    )
                    .clicked()
                {
                    self.category = cat;
                    self.results = demo_results(self.category);
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
                    egui::RichText::new("No results")
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
        let m = self
            .selected
            .and_then(|(gi, mi)| self.results.get(gi)?.matches.get(mi));
        let Some(m) = m else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select a result to preview")
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            });
            return;
        };
        let sp = preview_for(m, self.mode);
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
            for (i, line) in sp.source_lines.iter().enumerate() {
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
                    if is_match {
                        let job = highlight_preview(line, &sp.highlight_ranges);
                        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                        painter.galley(
                            egui::pos2(text_x, mid_y - galley.size().y / 2.0),
                            galley,
                            egui::Color32::PLACEHOLDER,
                        );
                    } else {
                        painter.text(
                            egui::pos2(text_x, mid_y),
                            egui::Align2::LEFT_CENTER,
                            line,
                            mono.clone(),
                            theme::TEXT_SECONDARY,
                        );
                    }
                });
            }
        });
    }
}

// -- 工具函数 --

fn preview_for(m: &SearchMatch, mode: SearchMode) -> &SourcePreview {
    match mode {
        SearchMode::Decompiled => &m.decompiled,
        SearchMode::Bytecode => &m.bytecode,
    }
}

/// 模式切换按钮（Decompiled / Bytecode）
fn mode_button(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let color = if active {
        theme::VERDIGRIS
    } else {
        theme::TEXT_MUTED
    };
    let fill = if active {
        theme::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(11.0).color(color))
            .fill(fill)
            .corner_radius(3)
            .min_size(egui::vec2(0.0, 22.0)),
    )
    .clicked()
}

fn toggle_icon(ui: &mut egui::Ui, icon: &str, active: bool, tooltip: &str) -> bool {
    let color = if active {
        theme::VERDIGRIS
    } else {
        theme::TEXT_MUTED
    };
    let fill = if active {
        theme::BG_LIGHT
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke = if active {
        egui::Stroke::new(1.0, theme::BORDER_LIGHT)
    } else {
        egui::Stroke::NONE
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(icon)
                .font(egui::FontId::new(15.0, codicon::family()))
                .color(color),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(4)
        .min_size(egui::vec2(28.0, 24.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}

fn render_group_header(ui: &mut egui::Ui, group: &SearchResultGroup) -> bool {
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(avail_w, GROUP_HEADER_HEIGHT),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect, 0.0, theme::BG_HOVER);
    }
    let mid_y = rect.center().y;
    let chevron = if group.expanded {
        codicon::CHEVRON_DOWN
    } else {
        codicon::CHEVRON_RIGHT
    };
    painter.text(
        egui::pos2(rect.left() + 8.0, mid_y),
        egui::Align2::LEFT_CENTER,
        chevron,
        egui::FontId::new(12.0, codicon::family()),
        theme::TEXT_MUTED,
    );
    painter.text(
        egui::pos2(rect.left() + 24.0, mid_y),
        egui::Align2::LEFT_CENTER,
        codicon::SYMBOL_CLASS,
        egui::FontId::new(12.0, codicon::family()),
        theme::VERDIGRIS,
    );
    painter.text(
        egui::pos2(rect.left() + 40.0, mid_y),
        egui::Align2::LEFT_CENTER,
        &group.class_name,
        egui::FontId::proportional(12.0),
        theme::TEXT_PRIMARY,
    );
    let info = format!("{}  ({} matches)", group.package, group.matches.len());
    painter.text(
        egui::pos2(rect.right() - 8.0, mid_y),
        egui::Align2::RIGHT_CENTER,
        &info,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    resp.clicked()
}

fn render_match_row(ui: &mut egui::Ui, m: &SearchMatch, selected: bool, mode: SearchMode) -> bool {
    let sp = preview_for(m, mode);
    let avail_w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(avail_w, ROW_HEIGHT), egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 0.0, theme::BG_HOVER);
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, theme::BG_LIGHT);
    }
    let mid_y = rect.center().y;
    let loc_x = rect.left() + 32.0;
    painter.text(
        egui::pos2(loc_x, mid_y),
        egui::Align2::LEFT_CENTER,
        &m.location,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    let loc_galley = painter.layout_no_wrap(
        m.location.clone(),
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    let preview_x = loc_x + loc_galley.size().x + 8.0;
    let job = highlight_preview(&sp.preview, &sp.highlight_ranges);
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    painter.galley(
        egui::pos2(preview_x, mid_y - galley.size().y / 2.0),
        galley,
        egui::Color32::PLACEHOLDER,
    );
    resp.clicked()
}

fn highlight_preview(text: &str, ranges: &[(usize, usize)]) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let bytes = text.as_bytes();
    let mut pos = 0;
    for &(start, end) in ranges {
        let start = start.min(bytes.len());
        let end = end.min(bytes.len());
        if start > pos {
            job.append(
                &text[pos..start],
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(11.0),
                    color: theme::TEXT_SECONDARY,
                    ..Default::default()
                },
            );
        }
        if end > start {
            job.append(
                &text[start..end],
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(11.0),
                    color: theme::VERDIGRIS,
                    ..Default::default()
                },
            );
        }
        pos = end;
    }
    if pos < text.len() {
        job.append(
            &text[pos..],
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(11.0),
                color: theme::TEXT_SECONDARY,
                ..Default::default()
            },
        );
    }
    job
}

fn separator(ui: &mut egui::Ui) {
    let avail = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(avail.left(), avail.top()),
            egui::pos2(avail.right(), avail.top()),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
    ui.allocate_space(egui::vec2(avail.width(), 1.0));
}

// ===========================
// Demo 数据：字节码 + 反编译
// ===========================

fn lines(src: &[&str]) -> Vec<String> {
    src.iter().map(|s| s.to_string()).collect()
}

fn sp(preview: &str, hl: Vec<(usize, usize)>, src: Vec<String>, line: usize) -> SourcePreview {
    SourcePreview {
        preview: preview.into(),
        highlight_ranges: hl,
        source_lines: src,
        match_line: line,
    }
}

// -- Bytecode sources --

fn bc_load_worlds() -> Vec<String> {
    lines(&[
        "// Method: loadWorlds()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  LDC \"overworld\"",
        "  ASTORE 2",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L1",
        "  ALOAD 0",
        "  LDC \"the_nether\"",
        "  ASTORE 3",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L2",
        "  ALOAD 0",
        "  LDC \"the_end\"",
        "  ASTORE 4",
        "  INVOKEVIRTUAL register (Ljava/lang/String;)V",
        "L3",
        "  RETURN",
    ])
}

fn bc_dedicated_init() -> Vec<String> {
    lines(&[
        "// Method: <init>()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  INVOKESPECIAL java/lang/Object.<init> ()V",
        "  ALOAD 0",
        "  LDC \"Starting minecraft server\"",
        "  INVOKESTATIC org/slf4j/LoggerFactory.getLogger ()V",
        "  ALOAD 0",
        "  ICONST_0",
        "  PUTFIELD running : Z",
        "L1",
        "  RETURN",
    ])
}

fn bc_dedicated_init_server() -> Vec<String> {
    lines(&[
        "// Method: initServer()Z",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  LDC \"server.properties\"",
        "  ASTORE 1",
        "  NEW java/io/File",
        "  DUP",
        "  ALOAD 1",
        "  INVOKESPECIAL java/io/File.<init> (Ljava/lang/String;)V",
        "  ASTORE 2",
        "L1",
        "  ALOAD 2",
        "  INVOKEVIRTUAL java/io/File.exists ()Z",
        "  IRETURN",
    ])
}

fn bc_mc_init() -> Vec<String> {
    lines(&[
        "// Method: <init>()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  INVOKESPECIAL java/lang/Object.<init> ()V",
        "  ALOAD 0",
        "  ACONST_NULL",
        "  PUTFIELD serverThread : Ljava/lang/Thread;",
        "  ALOAD 0",
        "  NEW java/util/ArrayList",
        "  DUP",
        "  INVOKESPECIAL java/util/ArrayList.<init> ()V",
        "  PUTFIELD levels : Ljava/util/List;",
        "  ALOAD 0",
        "  ICONST_0",
        "  PUTFIELD running : Z",
        "L1",
        "  RETURN",
    ])
}

fn bc_start_server() -> Vec<String> {
    lines(&[
        "// Method: startServer()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  ICONST_1",
        "  PUTFIELD running : Z",
        "  ALOAD 0",
        "  INVOKEVIRTUAL loadWorlds ()V",
        "  ALOAD 0",
        "  INVOKEVIRTUAL tickServer ()V",
        "L1",
        "  RETURN",
    ])
}

fn bc_tick_server() -> Vec<String> {
    lines(&[
        "// Method: tickServer()V",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  INVOKEINTERFACE java/util/List.iterator ()Ljava/util/Iterator;",
        "  ASTORE 1",
        "L1",
        "  ALOAD 1",
        "  INVOKEINTERFACE java/util/Iterator.hasNext ()Z",
        "  IFEQ L3",
        "  ALOAD 1",
        "  INVOKEINTERFACE java/util/Iterator.next ()Ljava/lang/Object;",
        "  CHECKCAST net/minecraft/server/ServerLevel",
        "  ASTORE 2",
        "  ALOAD 2",
        "  INVOKEVIRTUAL net/minecraft/server/ServerLevel.tick ()V",
        "L2",
        "  GOTO L1",
        "L3",
        "  RETURN",
    ])
}

fn bc_get_level_count() -> Vec<String> {
    lines(&[
        "// Method: getLevelCount()I",
        "// Access: public",
        "L0",
        "  ALOAD 0",
        "  GETFIELD levels : Ljava/util/List;",
        "  INVOKEINTERFACE java/util/List.size ()I",
        "  IRETURN",
    ])
}

fn bc_game_rules_clinit() -> Vec<String> {
    lines(&[
        "// Method: <clinit>()V",
        "// Access: static",
        "L0",
        "  GETSTATIC RANDOM_TICK_SPEED : LGameRule;",
        "  BIPUSH 20",
        "  INVOKEVIRTUAL setValue (I)V",
        "L1",
        "  GETSTATIC MAX_COMMAND_CHAIN : LGameRule;",
        "  SIPUSH 256",
        "  INVOKEVIRTUAL setValue (I)V",
        "L2",
        "  GETSTATIC DO_FIRE_TICK : LGameRule;",
        "  ICONST_1",
        "  INVOKEVIRTUAL setValue (Z)V",
        "L3",
        "  RETURN",
    ])
}

fn bc_mc_class_decl() -> Vec<String> {
    lines(&[
        "// net.minecraft.server.MinecraftServer",
        "// Access: public abstract",
        "// Superclass: java/lang/Object",
        "// Interfaces: java/lang/Runnable",
        "",
        "// Fields",
        "private final Thread serverThread",
        "private final List levels",
        "private volatile boolean running",
        "",
        "// Methods",
        "public void <init>()",
        "public void startServer()",
        "public void loadWorlds()",
        "public void tickServer()",
        "public int getLevelCount()",
    ])
}

fn bc_server_level_decl() -> Vec<String> {
    lines(&[
        "// net.minecraft.server.ServerLevel",
        "// Access: public",
        "// Superclass: net/minecraft/world/level/Level",
        "",
        "// Fields",
        "private final MinecraftServer server",
        "",
        "// Methods",
        "public void <init>(MinecraftServer)",
        "public void tick()",
        "public MinecraftServer getServer()",
    ])
}

// -- Decompiled (Java) sources --

fn dc_load_worlds() -> Vec<String> {
    lines(&[
        "public void loadWorlds() {",
        "    this.levels.clear();",
        "    ServerLevel overworld = new ServerLevel(this, \"overworld\");",
        "    this.register(overworld);",
        "    ServerLevel nether = new ServerLevel(this, \"the_nether\");",
        "    this.register(nether);",
        "    ServerLevel end = new ServerLevel(this, \"the_end\");",
        "    this.register(end);",
        "}",
    ])
}

fn dc_dedicated_init() -> Vec<String> {
    lines(&[
        "public DedicatedServer() {",
        "    super();",
        "    LOGGER.info(\"Starting minecraft server\");",
        "    this.running = false;",
        "}",
    ])
}

fn dc_dedicated_init_server() -> Vec<String> {
    lines(&[
        "public boolean initServer() {",
        "    String path = \"server.properties\";",
        "    File file = new File(path);",
        "    return file.exists();",
        "}",
    ])
}

fn dc_mc_init() -> Vec<String> {
    lines(&[
        "public MinecraftServer() {",
        "    super();",
        "    this.serverThread = null;",
        "    this.levels = new ArrayList<>();",
        "    this.running = false;",
        "}",
    ])
}

fn dc_start_server() -> Vec<String> {
    lines(&[
        "public void startServer() {",
        "    this.running = true;",
        "    this.loadWorlds();",
        "    this.tickServer();",
        "}",
    ])
}

fn dc_tick_server() -> Vec<String> {
    lines(&[
        "public void tickServer() {",
        "    for (ServerLevel level : this.levels) {",
        "        level.tick();",
        "    }",
        "}",
    ])
}

fn dc_get_level_count() -> Vec<String> {
    lines(&[
        "public int getLevelCount() {",
        "    return this.levels.size();",
        "}",
    ])
}

fn dc_game_rules_clinit() -> Vec<String> {
    lines(&[
        "static {",
        "    RANDOM_TICK_SPEED.setValue(20);",
        "    MAX_COMMAND_CHAIN.setValue(256);",
        "    DO_FIRE_TICK.setValue(true);",
        "}",
    ])
}

fn dc_mc_class() -> Vec<String> {
    lines(&[
        "public abstract class MinecraftServer implements Runnable {",
        "",
        "    private final Thread serverThread;",
        "    private final List<ServerLevel> levels;",
        "    private volatile boolean running;",
        "",
        "    public MinecraftServer() { ... }",
        "    public void startServer() { ... }",
        "    public void loadWorlds() { ... }",
        "    public void tickServer() { ... }",
        "    public int getLevelCount() { ... }",
        "}",
    ])
}

fn dc_server_level_class() -> Vec<String> {
    lines(&[
        "public class ServerLevel extends Level {",
        "",
        "    private final MinecraftServer server;",
        "",
        "    public ServerLevel(MinecraftServer server) { ... }",
        "    public void tick() { ... }",
        "    public MinecraftServer getServer() { ... }",
        "}",
    ])
}

// -- Demo results --

fn demo_results(category: SearchCategory) -> Vec<SearchResultGroup> {
    match category {
        SearchCategory::Strings => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        //                     01234567890123
                        bytecode: sp("LDC \"overworld\"", vec![(5, 14)], bc_load_worlds(), 5),
                        //                                              0         1         2         3         4         5
                        //                                              0123456789012345678901234567890123456789012345678901234567
                        decompiled: sp(
                            "ServerLevel overworld = new ServerLevel(this, \"overworld\");",
                            vec![(46, 55)],
                            dc_load_worlds(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp("LDC \"the_nether\"", vec![(5, 15)], bc_load_worlds(), 10),
                        decompiled: sp(
                            "ServerLevel nether = new ServerLevel(this, \"the_nether\");",
                            vec![(43, 53)],
                            dc_load_worlds(),
                            4,
                        ),
                    },
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp("LDC \"the_end\"", vec![(5, 12)], bc_load_worlds(), 15),
                        decompiled: sp(
                            "ServerLevel end = new ServerLevel(this, \"the_end\");",
                            vec![(40, 47)],
                            dc_load_worlds(),
                            6,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "DedicatedServer".into(),
                package: "net.minecraft.server.dedicated".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "<init>()".into(),
                        bytecode: sp(
                            "LDC \"Starting minecraft server\"",
                            vec![(5, 30)],
                            bc_dedicated_init(),
                            6,
                        ),
                        decompiled: sp(
                            "LOGGER.info(\"Starting minecraft server\");",
                            vec![(12, 39)],
                            dc_dedicated_init(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "initServer()".into(),
                        bytecode: sp(
                            "LDC \"server.properties\"",
                            vec![(5, 22)],
                            bc_dedicated_init_server(),
                            4,
                        ),
                        decompiled: sp(
                            "String path = \"server.properties\";",
                            vec![(15, 32)],
                            dc_dedicated_init_server(),
                            1,
                        ),
                    },
                ],
            },
        ],
        SearchCategory::Values => vec![SearchResultGroup {
            class_name: "GameRules".into(),
            package: "net.minecraft.world.level".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "<clinit>()".into(),
                    bytecode: sp("BIPUSH 20", vec![(7, 9)], bc_game_rules_clinit(), 4),
                    decompiled: sp(
                        "RANDOM_TICK_SPEED.setValue(20);",
                        vec![(26, 28)],
                        dc_game_rules_clinit(),
                        1,
                    ),
                },
                SearchMatch {
                    location: "<clinit>()".into(),
                    bytecode: sp("SIPUSH 256", vec![(7, 10)], bc_game_rules_clinit(), 8),
                    decompiled: sp(
                        "MAX_COMMAND_CHAIN.setValue(256);",
                        vec![(26, 29)],
                        dc_game_rules_clinit(),
                        2,
                    ),
                },
            ],
        }],
        SearchCategory::ClassReferences => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "loadWorlds()".into(),
                        bytecode: sp(
                            "NEW net/minecraft/server/ServerLevel",
                            vec![(4, 35)],
                            bc_load_worlds(),
                            5,
                        ),
                        decompiled: sp(
                            "ServerLevel overworld = new ServerLevel(this, \"overworld\");",
                            vec![(28, 39)],
                            dc_load_worlds(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "tickServer()".into(),
                        bytecode: sp(
                            "CHECKCAST net/minecraft/server/ServerLevel",
                            vec![(10, 41)],
                            bc_tick_server(),
                            13,
                        ),
                        decompiled: sp(
                            "for (ServerLevel level : this.levels) {",
                            vec![(5, 16)],
                            dc_tick_server(),
                            1,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "ServerPlayer".into(),
                package: "net.minecraft.server".into(),
                expanded: false,
                matches: vec![SearchMatch {
                    location: "<init>()".into(),
                    bytecode: sp(
                        "INVOKESPECIAL net/minecraft/server/ServerLevel.<init>",
                        vec![(14, 52)],
                        bc_mc_init(),
                        4,
                    ),
                    decompiled: sp(
                        "this.levels = new ArrayList<>();",
                        vec![(18, 31)],
                        dc_mc_init(),
                        3,
                    ),
                }],
            },
        ],
        SearchCategory::MemberReferences => vec![SearchResultGroup {
            class_name: "MinecraftServer".into(),
            package: "net.minecraft.server".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "startServer()".into(),
                    bytecode: sp("PUTFIELD running : Z", vec![(9, 16)], bc_start_server(), 5),
                    decompiled: sp("this.running = true;", vec![(5, 12)], dc_start_server(), 1),
                },
                SearchMatch {
                    location: "startServer()".into(),
                    bytecode: sp(
                        "INVOKEVIRTUAL loadWorlds ()V",
                        vec![(14, 24)],
                        bc_start_server(),
                        7,
                    ),
                    decompiled: sp("this.loadWorlds();", vec![(5, 15)], dc_start_server(), 2),
                },
                SearchMatch {
                    location: "tickServer()".into(),
                    bytecode: sp(
                        "GETFIELD levels : Ljava/util/List;",
                        vec![(9, 15)],
                        bc_tick_server(),
                        4,
                    ),
                    decompiled: sp(
                        "for (ServerLevel level : this.levels) {",
                        vec![(29, 35)],
                        dc_tick_server(),
                        1,
                    ),
                },
            ],
        }],
        SearchCategory::MemberDeclarations => vec![
            SearchResultGroup {
                class_name: "MinecraftServer".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![
                    SearchMatch {
                        location: "field".into(),
                        bytecode: sp(
                            "private final Thread serverThread",
                            vec![(21, 33)],
                            bc_mc_class_decl(),
                            6,
                        ),
                        decompiled: sp(
                            "private final Thread serverThread;",
                            vec![(21, 33)],
                            dc_mc_class(),
                            2,
                        ),
                    },
                    SearchMatch {
                        location: "field".into(),
                        bytecode: sp(
                            "private final List levels",
                            vec![(20, 26)],
                            bc_mc_class_decl(),
                            7,
                        ),
                        decompiled: sp(
                            "private final List<ServerLevel> levels;",
                            vec![(32, 38)],
                            dc_mc_class(),
                            3,
                        ),
                    },
                    SearchMatch {
                        location: "method".into(),
                        bytecode: sp(
                            "public void startServer()",
                            vec![(12, 23)],
                            bc_mc_class_decl(),
                            12,
                        ),
                        decompiled: sp(
                            "public void startServer() { ... }",
                            vec![(12, 23)],
                            dc_mc_class(),
                            7,
                        ),
                    },
                ],
            },
            SearchResultGroup {
                class_name: "ServerLevel".into(),
                package: "net.minecraft.server".into(),
                expanded: true,
                matches: vec![SearchMatch {
                    location: "method".into(),
                    bytecode: sp(
                        "public void tick()",
                        vec![(12, 16)],
                        bc_server_level_decl(),
                        9,
                    ),
                    decompiled: sp(
                        "public void tick() { ... }",
                        vec![(12, 16)],
                        dc_server_level_class(),
                        5,
                    ),
                }],
            },
        ],
        SearchCategory::Instructions => vec![SearchResultGroup {
            class_name: "MinecraftServer".into(),
            package: "net.minecraft.server".into(),
            expanded: true,
            matches: vec![
                SearchMatch {
                    location: "<init>()".into(),
                    bytecode: sp(
                        "INVOKESPECIAL java/lang/Object.<init> ()V",
                        vec![(0, 13)],
                        bc_mc_init(),
                        4,
                    ),
                    decompiled: sp("super();", vec![(0, 7)], dc_mc_init(), 1),
                },
                SearchMatch {
                    location: "loadWorlds()".into(),
                    bytecode: sp(
                        "INVOKEVIRTUAL register (Ljava/lang/String;)V",
                        vec![(0, 13)],
                        bc_load_worlds(),
                        7,
                    ),
                    decompiled: sp(
                        "this.register(overworld);",
                        vec![(5, 13)],
                        dc_load_worlds(),
                        3,
                    ),
                },
                SearchMatch {
                    location: "getLevelCount()".into(),
                    bytecode: sp(
                        "INVOKEINTERFACE java/util/List.size ()I",
                        vec![(0, 15)],
                        bc_get_level_count(),
                        5,
                    ),
                    decompiled: sp(
                        "return this.levels.size();",
                        vec![(18, 22)],
                        dc_get_level_count(),
                        1,
                    ),
                },
            ],
        }],
    }
}
