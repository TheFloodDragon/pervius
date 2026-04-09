//! 右侧内容区：egui_dock 管理多 Tab + TextEdit 代码编辑
//!
//! @author sky

pub mod highlight;
pub mod view_toggle;

use crate::shell::{codicon, theme};
use eframe::egui;
use egui_dock::{DockArea, DockState, Style as DockStyle, TabViewer};
use highlight::Language;
use view_toggle::ActiveView;

/// 编辑器 Tab 数据
pub struct EditorTab {
    pub title: String,
    /// 反编译源码（只读）
    pub decompiled: String,
    /// 字节码文本（可编辑）
    pub bytecode: String,
    /// Hex dump 文本（只读）
    pub hex_dump: String,
    #[allow(dead_code)]
    pub language: Language,
    pub active_view: ActiveView,
    pub is_modified: bool,
    /// Decompiled 视图的 layouter 缓存
    layouter_decompiled: Box<dyn FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley>>,
}

impl EditorTab {
    pub fn new(
        title: impl Into<String>,
        decompiled: impl Into<String>,
        bytecode: impl Into<String>,
        hex_dump: impl Into<String>,
        language: Language,
    ) -> Self {
        let lang = language;
        Self {
            title: title.into(),
            decompiled: decompiled.into(),
            bytecode: bytecode.into(),
            hex_dump: hex_dump.into(),
            language,
            active_view: ActiveView::Decompiled,
            is_modified: false,
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
        }
    }
}

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
        let style = build_dock_style(ui.style());
        DockArea::new(&mut self.dock_state)
            .style(style)
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show_inside(ui, &mut EditorTabViewer);
    }

    /// 添加新 Tab
    #[allow(dead_code)]
    pub fn open_tab(&mut self, tab: EditorTab) {
        self.dock_state.main_surface_mut().push_to_focused_leaf(tab);
    }

    /// 获取当前活跃 Tab 的 active_view（供 StatusBar 显示）
    pub fn focused_view(&mut self) -> Option<ActiveView> {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            return Some(tab.active_view);
        }
        let (_, tab) = self.dock_state.main_surface_mut().find_active()?;
        Some(tab.active_view)
    }

    /// 设置当前活跃 Tab 的 active_view（从 StatusBar 切换）
    pub fn set_focused_view(&mut self, view: ActiveView) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            tab.active_view = view;
            return;
        }
        if let Some((_, tab)) = self.dock_state.main_surface_mut().find_active() {
            tab.active_view = view;
        }
    }
}

/// TabViewer 实现：定义每个 Tab 的标题和内容
struct EditorTabViewer;

impl TabViewer for EditorTabViewer {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let mut job = egui::text::LayoutJob::default();
        // class 图标
        job.append(
            codicon::SYMBOL_CLASS,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(11.0, codicon::family()),
                color: theme::VERDIGRIS,
                ..Default::default()
            },
        );
        job.append(" ", 0.0, egui::TextFormat::default());
        // 标题文字
        job.append(
            &tab.title,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: theme::TEXT_PRIMARY,
                ..Default::default()
            },
        );
        // 修改标记
        if tab.is_modified {
            job.append(" ", 0.0, egui::TextFormat::default());
            job.append(
                codicon::CIRCLE_FILLED,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(6.0),
                    color: theme::ACCENT_ORANGE,
                    ..Default::default()
                },
            );
        }
        job.into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        match tab.active_view {
            ActiveView::Decompiled => render_decompiled(ui, tab),
            ActiveView::Bytecode => render_bytecode(ui, tab),
            ActiveView::Hex => render_hex(ui, tab),
        }
    }
}

/// 行号栏宽度（根据行数计算位数）
fn line_number_width(line_count: usize) -> f32 {
    let digits = if line_count == 0 {
        1
    } else {
        (line_count as f32).log10().floor() as usize + 1
    };
    // 每位约 8px（monospace 13），加左右各 12px padding
    digits as f32 * 8.0 + 24.0
}

/// 在 TextEdit 左侧绘制行号
fn paint_line_numbers(ui: &egui::Ui, text_rect: egui::Rect, text: &str, gutter_w: f32) {
    let painter = ui.painter();
    let font = egui::FontId::monospace(13.0);
    let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
    let gutter_rect = egui::Rect::from_min_size(
        text_rect.left_top(),
        egui::vec2(gutter_w, text_rect.height()),
    );
    // 行号栏背景（略深于编辑区）
    painter.rect_filled(gutter_rect, 0.0, theme::BG_DARK);
    let line_count = text.lines().count().max(1);
    let top_y = text_rect.top() + 4.0;
    for i in 0..line_count {
        let y = top_y + i as f32 * line_height;
        if y > text_rect.bottom() {
            break;
        }
        painter.text(
            egui::pos2(gutter_rect.right() - 8.0, y),
            egui::Align2::RIGHT_TOP,
            format!("{}", i + 1),
            font.clone(),
            theme::TEXT_MUTED,
        );
    }
}

/// 反编译视图：只读，带语法高亮
fn render_decompiled(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let layouter = &mut tab.layouter_decompiled;
    let line_count = tab.decompiled.lines().count().max(1);
    let gutter_w = line_number_width(line_count);
    let min = egui::vec2(ui.available_width(), ui.available_height());
    let response = ui.add(
        egui::TextEdit::multiline(&mut tab.decompiled)
            .id_salt(format!("te_dec_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_MEDIUM)
                    .inner_margin(egui::Margin {
                        left: gutter_w as i8,
                        right: 8,
                        top: 4,
                        bottom: 4,
                    }),
            )
            .interactive(false)
            .min_size(min)
            .desired_width(f32::INFINITY)
            .layouter(&mut |ui, text, wrap_width| layouter(ui, text.as_str(), wrap_width)),
    );
    paint_line_numbers(ui, response.rect, &tab.decompiled, gutter_w);
}

/// 字节码视图：可编辑纯文本
fn render_bytecode(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let line_count = tab.bytecode.lines().count().max(1);
    let gutter_w = line_number_width(line_count);
    let min = egui::vec2(ui.available_width(), ui.available_height());
    let response = ui.add(
        egui::TextEdit::multiline(&mut tab.bytecode)
            .id_salt(format!("te_bc_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_MEDIUM)
                    .inner_margin(egui::Margin {
                        left: gutter_w as i8,
                        right: 8,
                        top: 4,
                        bottom: 4,
                    }),
            )
            .min_size(min)
            .desired_width(f32::INFINITY),
    );
    paint_line_numbers(ui, response.rect, &tab.bytecode, gutter_w);
}

/// Hex 视图：只读 hex dump（Hex 视图不显示行号）
fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let min = egui::vec2(ui.available_width(), ui.available_height());
    ui.add(
        egui::TextEdit::multiline(&mut tab.hex_dump)
            .id_salt(format!("te_hex_{}", tab.title))
            .font(egui::FontId::monospace(13.0))
            .code_editor()
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_MEDIUM)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .interactive(false)
            .min_size(min)
            .desired_width(f32::INFINITY),
    );
}

/// 构建匹配主题的 dock 样式
fn build_dock_style(egui_style: &egui::Style) -> DockStyle {
    let mut style = DockStyle::from_egui(egui_style);
    // Tab 栏背景
    style.tab_bar.bg_fill = theme::BG_DARK;
    // 活跃 Tab 不显示底部分隔线（视觉上与内容区融合）
    style.tab.hline_below_active_tab_name = false;
    // 活跃 Tab
    style.tab.active.bg_fill = theme::BG_MEDIUM;
    style.tab.active.text_color = theme::TEXT_PRIMARY;
    // 非活跃 Tab
    style.tab.inactive.bg_fill = theme::BG_DARK;
    style.tab.inactive.text_color = theme::TEXT_SECONDARY;
    // 获得焦点的 Tab
    style.tab.focused.bg_fill = theme::BG_MEDIUM;
    style.tab.focused.text_color = theme::TEXT_PRIMARY;
    // tab body 边框和内边距清零，背景色与编辑器一致
    style.tab.tab_body.stroke = egui::Stroke::NONE;
    style.tab.tab_body.inner_margin = egui::Margin::ZERO;
    style.tab.tab_body.bg_fill = theme::BG_MEDIUM;
    // 分隔线
    style.separator.color_idle = theme::BORDER;
    style.separator.color_hovered = theme::VERDIGRIS;
    style.separator.color_dragged = theme::VERDIGRIS;
    style
}
