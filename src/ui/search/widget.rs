//! 搜索面板的小型 widget 和渲染辅助函数
//!
//! @author sky

use super::result::{SearchMatch, SearchResultGroup, SourcePreview};
use crate::appearance::{codicon, theme};
use eframe::egui;
use rust_i18n::t;

/// 结果行高
pub const ROW_HEIGHT: f32 = 24.0;
/// 分组 header 行高
pub const GROUP_HEADER_HEIGHT: f32 = 28.0;

/// 预览视图模式
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Decompiled,
    Bytecode,
}

pub fn preview_for(m: &SearchMatch, mode: SearchMode) -> &SourcePreview {
    match mode {
        SearchMode::Decompiled => &m.decompiled,
        SearchMode::Bytecode => &m.bytecode,
    }
}

pub fn render_group_header(ui: &mut egui::Ui, group: &SearchResultGroup) -> bool {
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
        codicon::JAVA,
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
    let info = format!(
        "{}  ({})",
        group.package,
        t!("search.matches", count = group.matches.len())
    );
    painter.text(
        egui::pos2(rect.right() - 8.0, mid_y),
        egui::Align2::RIGHT_CENTER,
        &info,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    resp.clicked()
}

pub fn render_match_row(
    ui: &mut egui::Ui,
    m: &SearchMatch,
    selected: bool,
    mode: SearchMode,
) -> bool {
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

/// 结果列表行内匹配区间高亮（VERDIGRIS 前景）
fn highlight_preview(text: &str, ranges: &[(usize, usize)]) -> egui::text::LayoutJob {
    let fmt = |color| egui::TextFormat {
        font_id: egui::FontId::monospace(11.0),
        color,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    let len = text.len();
    let mut pos = 0;
    for &(start, end) in ranges {
        let start = start.min(len);
        let end = end.min(len);
        if start > pos {
            job.append(&text[pos..start], 0.0, fmt(theme::TEXT_SECONDARY));
        }
        if end > start {
            job.append(&text[start..end], 0.0, fmt(theme::VERDIGRIS));
        }
        pos = end;
    }
    if pos < len {
        job.append(&text[pos..], 0.0, fmt(theme::TEXT_SECONDARY));
    }
    job
}

pub fn separator(ui: &mut egui::Ui) {
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
