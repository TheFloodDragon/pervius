//! 搜索框 + 搜索结果行
//!
//! @author sky

use crate::shell::{codicon, theme};
use eframe::egui;

/// 搜索结果
pub struct SearchResult {
    pub file_path: String,
    pub line_num: u32,
    pub preview: String,
}

/// 渲染搜索输入框（占位，暂不可输入）
pub fn search_box(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let avail = ui.available_width() - 6.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(avail, 28.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 6.0, theme::BG_MEDIUM);
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Middle,
        );
        painter.text(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            codicon::SEARCH,
            egui::FontId::new(12.0, codicon::family()),
            theme::TEXT_MUTED,
        );
        painter.text(
            egui::pos2(rect.left() + 28.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Search in all classes...",
            egui::FontId::proportional(12.0),
            theme::TEXT_MUTED,
        );
    });
}

/// 渲染单条搜索结果
pub fn search_row(ui: &mut egui::Ui, result: &SearchResult) {
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, 40.0), egui::Sense::click());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 4.0, theme::BG_HOVER);
    }
    // 文件路径
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.top() + 12.0),
        egui::Align2::LEFT_CENTER,
        &result.file_path,
        egui::FontId::proportional(11.0),
        theme::VERDIGRIS,
    );
    // 行号
    let path_w = painter
        .layout_no_wrap(
            result.file_path.clone(),
            egui::FontId::proportional(11.0),
            theme::VERDIGRIS,
        )
        .size()
        .x;
    painter.text(
        egui::pos2(rect.left() + 12.0 + path_w + 4.0, rect.top() + 12.0),
        egui::Align2::LEFT_CENTER,
        format!(":{}", result.line_num),
        egui::FontId::proportional(11.0),
        theme::ACCENT_ORANGE,
    );
    // 预览
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.bottom() - 12.0),
        egui::Align2::LEFT_CENTER,
        &result.preview,
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );
}
