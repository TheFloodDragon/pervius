//! 编辑器视图渲染：代码视图（TextEdit read-only）+ Hex 视图
//!
//! 使用 egui 原生 TextEdit（`&str` TextBuffer，只读）实现代码视图，
//! 通过 layouter 注入语法高亮和搜索匹配高亮。
//! 行号栏通过 painter overlay 绘制在视口左侧固定位置。
//!
//! @author sky

use super::find::FindMatch;
use super::highlight;
use super::tab::EditorTab;
use crate::shell::theme;
use eframe::egui;
use std::sync::Arc;

/// 行号栏右侧到文本的间距
const GUTTER_PAD: f32 = 8.0;
/// 行内左侧 padding
const TEXT_PAD_LEFT: f32 = 8.0;
/// 代码字体大小
const CODE_FONT_SIZE: f32 = 13.0;

/// 行号栏宽度（根据最大行号计算位数）
pub fn line_number_width(max_number: usize) -> f32 {
    let digits = if max_number == 0 {
        1
    } else {
        (max_number as f32).log10().floor() as usize + 1
    };
    digits as f32 * 8.0 + 24.0
}

/// 在 ScrollArea 外绘制全高背景（左侧 gutter + 右侧编辑区）
pub fn paint_editor_bg(ui: &egui::Ui, full_rect: egui::Rect, gutter_w: f32) {
    let painter = ui.painter();
    painter.rect_filled(full_rect, 0.0, theme::BG_DARKEST);
    painter.rect_filled(
        egui::Rect::from_min_size(
            full_rect.left_top(),
            egui::vec2(gutter_w, full_rect.height()),
        ),
        0.0,
        theme::BG_GUTTER,
    );
}

/// 代码视图（read-only TextEdit + 语法高亮 + 搜索高亮 + 行号）
///
/// `line_mapping` 为空时显示顺序行号（1-indexed），
/// 非空时显示原始源码行号，无映射行不显示行号。
fn render_code_view(
    ui: &mut egui::Ui,
    text: &str,
    spans: &[highlight::Span],
    line_mapping: &[Option<u32>],
    matches: &[FindMatch],
    current: Option<usize>,
) {
    let line_count = text.split('\n').count().max(1);
    let max_number = if line_mapping.is_empty() {
        line_count
    } else {
        line_mapping
            .iter()
            .filter_map(|n| n.map(|v| v as usize))
            .max()
            .unwrap_or(line_count)
    };
    let gutter_w = line_number_width(max_number);
    let code_font = egui::FontId::monospace(CODE_FONT_SIZE);
    let match_ranges: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    let match_ref = match_ranges.as_slice();
    // layouter: 语法高亮 + 搜索匹配背景
    let mut layouter =
        |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| -> Arc<egui::Galley> {
            let s = text_buf.as_str();
            let job = highlight::build_layout_job_with_matches(s, spans, match_ref, current);
            ui.fonts_mut(|f| f.layout_job(job))
        };
    // TextEdit（read-only &str buffer）
    let mut galley_y = 0.0f32;
    ui.horizontal_top(|ui| {
        ui.add_space(gutter_w + GUTTER_PAD + TEXT_PAD_LEFT);
        let mut buf: &str = text;
        let output = egui::TextEdit::multiline(&mut buf)
            .desired_width(f32::INFINITY)
            .font(code_font.clone())
            .frame(egui::Frame::NONE)
            .layouter(&mut layouter)
            .show(ui);
        galley_y = output.galley_pos.y;
    });
    // 行号 overlay（固定在视口左侧，不随水平滚动移动）
    let clip = ui.clip_rect();
    let painter = ui.painter();
    let measure = painter.layout_no_wrap("M".to_string(), code_font.clone(), egui::Color32::WHITE);
    let line_height = measure.size().y;
    // 行号区背景（覆盖水平滚动时溢出到 gutter 区域的代码）
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(clip.left(), clip.top()),
            egui::vec2(gutter_w + GUTTER_PAD, clip.height()),
        ),
        0.0,
        theme::BG_GUTTER,
    );
    let gutter_right_x = clip.left() + gutter_w;
    let first = ((clip.top() - galley_y) / line_height).max(0.0) as usize;
    let last = ((clip.bottom() - galley_y) / line_height + 1.0)
        .ceil()
        .min(line_count as f32) as usize;
    for i in first..last {
        let y = galley_y + i as f32 * line_height;
        let line_label: Option<usize> = if line_mapping.is_empty() {
            Some(i + 1)
        } else {
            line_mapping.get(i).and_then(|n| n.map(|v| v as usize))
        };
        if let Some(num) = line_label {
            painter.text(
                egui::pos2(gutter_right_x - 8.0, y),
                egui::Align2::RIGHT_TOP,
                num,
                code_font.clone(),
                theme::TEXT_MUTED,
            );
        }
    }
}

/// 反编译视图
pub fn render_decompiled(
    ui: &mut egui::Ui,
    tab: &mut EditorTab,
    matches: &[FindMatch],
    current: Option<usize>,
) {
    render_code_view(
        ui,
        &tab.decompiled,
        &tab.decompiled_data.spans,
        &tab.decompiled_line_mapping,
        matches,
        current,
    );
}

/// Hex 视图
pub fn render_hex(ui: &mut egui::Ui, tab: &mut EditorTab) {
    let theme = super::style::hex::hex_theme();
    egui_hex_view::show(ui, &tab.raw_bytes, &mut tab.hex_state, &theme);
}
