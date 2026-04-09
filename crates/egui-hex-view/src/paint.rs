//! 绘制：列头、数据行
//!
//! @author sky

use crate::layout::Cols;
use crate::{HexTheme, HexViewState, BYTES_PER_ROW, FONT_SIZE, HEADER_H, PAD_TOP, ROW_H};
use eframe::egui;

/// 字节值 → hex 区颜色
fn byte_hex_color(b: u8, theme: &HexTheme) -> egui::Color32 {
    match b {
        0x00 => theme.hex_null_color,
        0x20..=0x7E => theme.hex_printable_color,
        0x01..=0x1F | 0x7F => theme.hex_control_color,
        _ => theme.hex_high_color,
    }
}

// -- 列头 --

pub(crate) fn header(ui: &mut egui::Ui, cols: &Cols, theme: &HexTheme, total_w: f32) {
    let (response, painter) =
        ui.allocate_painter(egui::vec2(total_w, HEADER_H), egui::Sense::hover());
    let origin = response.rect.min;
    let font = egui::FontId::monospace(FONT_SIZE);
    let cy = origin.y + HEADER_H * 0.5;
    painter.rect_filled(response.rect, 0.0, theme.header_bg);
    painter.text(
        egui::pos2(origin.x + cols.addr_x, cy),
        egui::Align2::LEFT_CENTER,
        "Offset",
        font.clone(),
        theme.header_color,
    );
    for col in 0..BYTES_PER_ROW {
        painter.text(
            egui::pos2(origin.x + cols.hex_byte_x(col), cy),
            egui::Align2::LEFT_CENTER,
            format!("{col:02X}"),
            font.clone(),
            theme.header_color,
        );
    }
    painter.text(
        egui::pos2(origin.x + cols.ascii_x, cy),
        egui::Align2::LEFT_CENTER,
        "Decoded text",
        font.clone(),
        theme.header_color,
    );
    // 底部分隔线
    painter.line_segment(
        [
            egui::pos2(origin.x, origin.y + HEADER_H - 1.0),
            egui::pos2(origin.x + total_w, origin.y + HEADER_H - 1.0),
        ],
        egui::Stroke::new(1.0, theme.border),
    );
}

// -- 数据行 --

pub(crate) fn row(
    painter: &egui::Painter,
    cols: &Cols,
    font: &egui::FontId,
    origin: egui::Pos2,
    row: usize,
    data: &[u8],
    state: &HexViewState,
    hover_idx: Option<usize>,
    hover_row: Option<usize>,
    theme: &HexTheme,
    content_w: f32,
) {
    let row_offset = row * BYTES_PER_ROW;
    let row_end = (row_offset + BYTES_PER_ROW).min(data.len());
    let y = origin.y + PAD_TOP + row as f32 * ROW_H;
    let cy = y + ROW_H * 0.5;
    let row_rect = egui::Rect::from_min_size(egui::pos2(origin.x, y), egui::vec2(content_w, ROW_H));
    if hover_row == Some(row) {
        painter.rect_filled(row_rect, 0.0, theme.hover_row_bg);
    }
    // 地址
    let addr_color = if hover_row == Some(row) {
        theme.addr_hover_color
    } else {
        theme.addr_color
    };
    painter.text(
        egui::pos2(origin.x + cols.addr_x, cy),
        egui::Align2::LEFT_CENTER,
        format!("{row_offset:08X}"),
        font.clone(),
        addr_color,
    );
    // Hex 字节 + ASCII
    for col in 0..(row_end - row_offset) {
        let byte_idx = row_offset + col;
        let b = data[byte_idx];
        let is_cursor = state.cursor == Some(byte_idx);
        let is_selected = state
            .selection
            .map_or(false, |(s, e)| byte_idx >= s && byte_idx < e);
        let is_hover = hover_idx == Some(byte_idx) && !is_cursor && !is_selected;
        let hex_x = origin.x + cols.hex_byte_x(col);
        let ascii_x = origin.x + cols.ascii_byte_x(col);
        // 背景高亮
        if is_cursor || is_selected || is_hover {
            let bg = if is_cursor {
                theme.cursor_bg
            } else if is_selected {
                theme.selection_bg
            } else {
                theme.hover_byte_bg
            };
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(hex_x, y),
                    egui::vec2(cols.char_w * 2.0, ROW_H),
                ),
                2.0,
                bg,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(ascii_x, y), egui::vec2(cols.char_w, ROW_H)),
                2.0,
                bg,
            );
        }
        // hex 文本
        let hex_color = if is_cursor {
            theme.text_primary
        } else {
            byte_hex_color(b, theme)
        };
        painter.text(
            egui::pos2(hex_x, cy),
            egui::Align2::LEFT_CENTER,
            format!("{b:02X}"),
            font.clone(),
            hex_color,
        );
        // ASCII 文本
        let ch = if b.is_ascii_graphic() || b == b' ' {
            b as char
        } else {
            '.'
        };
        let ascii_color = if is_cursor {
            theme.text_primary
        } else if ch == '.' {
            theme.ascii_dot_color
        } else {
            theme.ascii_color
        };
        painter.text(
            egui::pos2(ascii_x, cy),
            egui::Align2::LEFT_CENTER,
            String::from(ch),
            font.clone(),
            ascii_color,
        );
    }
}
