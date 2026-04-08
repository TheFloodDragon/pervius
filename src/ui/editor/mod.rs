//! 右侧内容区：Tab 栏 + ViewToggle + 代码编辑区
//!
//! @author sky

pub mod highlight;
pub mod tab;
pub mod view_toggle;

use crate::shell::theme;
use eframe::egui;
use highlight::{token_color, CodeLine};
use tab::TabInfo;
use view_toggle::ActiveView;

/// 代码行高度
const LINE_HEIGHT: f32 = 20.0;
/// 行号栏宽度
const GUTTER_WIDTH: f32 = 50.0;

/// 内容区域状态
pub struct ContentArea {
    pub tabs: Vec<TabInfo>,
    pub active_view: ActiveView,
    pub code_lines: Vec<CodeLine>,
}

impl ContentArea {
    pub fn new(tabs: Vec<TabInfo>, code_lines: Vec<CodeLine>) -> Self {
        Self {
            tabs,
            active_view: ActiveView::Decompiled,
            code_lines,
        }
    }

    /// 在给定 rect 内渲染
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let mut y = rect.top();
        // Tab 栏
        let tab_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left(), y),
            egui::vec2(rect.width(), theme::TAB_BAR_HEIGHT),
        );
        self.render_tabs(ui, tab_rect);
        y = tab_rect.bottom();
        Self::hsep(ui, rect.left(), y, rect.width());
        y += theme::BORDER_WIDTH;
        // ViewToggle
        let vt_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left(), y),
            egui::vec2(rect.width(), theme::VIEW_TOOLBAR_HEIGHT),
        );
        self.active_view = view_toggle::render(ui, vt_rect, self.active_view);
        y = vt_rect.bottom();
        Self::hsep(ui, rect.left(), y, rect.width());
        y += theme::BORDER_WIDTH;
        // 编辑区
        let editor_rect = egui::Rect::from_min_max(egui::pos2(rect.left(), y), rect.right_bottom());
        self.render_editor(ui, editor_rect);
    }

    fn render_editor(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        // 行号栏背景
        let gutter_rect =
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(GUTTER_WIDTH, rect.height()));
        painter.rect_filled(gutter_rect, 0.0, theme::BG_DARK);
        // 代码区背景
        let code_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + GUTTER_WIDTH, rect.top()),
            rect.right_bottom(),
        );
        painter.rect_filled(code_rect, 0.0, theme::BG_MEDIUM);
        // 滚动区域
        let mut scroll_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        egui::ScrollArea::vertical()
            .id_salt("code_editor")
            .show(&mut scroll_ui, |ui| {
                ui.add_space(8.0);
                for line in &self.code_lines {
                    self.render_code_line(ui, line, rect.left());
                }
                ui.add_space(8.0);
            });
    }

    fn render_code_line(&self, ui: &mut egui::Ui, line: &CodeLine, base_x: f32) {
        let avail_w = ui.available_width();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(avail_w, LINE_HEIGHT), egui::Sense::hover());
        let painter = ui.painter();
        let y = rect.center().y;
        // 行号（右对齐在 gutter 内）
        painter.text(
            egui::pos2(base_x + GUTTER_WIDTH - 8.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{}", line.line_num),
            egui::FontId::monospace(12.0),
            theme::TEXT_MUTED,
        );
        // token 序列
        let mut x = base_x + GUTTER_WIDTH + 12.0;
        for token in &line.tokens {
            if token.text.is_empty() {
                continue;
            }
            let color = token_color(token.color_id);
            let galley =
                painter.layout_no_wrap(token.text.clone(), egui::FontId::monospace(12.0), color);
            let w = galley.size().x;
            painter.galley(egui::pos2(x, y - galley.size().y / 2.0), galley, color);
            x += w;
        }
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.painter().rect_filled(rect, 0.0, theme::BG_DARK);
        let mut tab_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        tab_ui.horizontal_centered(|ui| {
            ui.add_space(4.0);
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut clicked_index = None;
            for (i, t) in self.tabs.iter().enumerate() {
                if tab::render(ui, t) {
                    clicked_index = Some(i);
                }
            }
            if let Some(idx) = clicked_index {
                for (i, t) in self.tabs.iter_mut().enumerate() {
                    t.is_active = i == idx;
                }
            }
        });
    }

    fn hsep(ui: &egui::Ui, x: f32, y: f32, w: f32) {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, theme::BORDER_WIDTH)),
            0.0,
            theme::BORDER,
        );
    }
}
