//! 列布局计算 + 字体测量
//!
//! @author sky

use crate::{ADDR_CHARS, BYTES_PER_ROW, FONT_SIZE, HEX_CHARS, PAD_LEFT, SECTION_GAP};
use eframe::egui;

/// 各列 x 坐标（由实测字符宽度计算）
pub(crate) struct Cols {
    pub addr_x: f32,
    pub hex_x: f32,
    pub ascii_x: f32,
    pub total_w: f32,
    pub sep1_x: f32,
    pub sep2_x: f32,
    pub char_w: f32,
    pub group_gap: f32,
}

impl Cols {
    pub fn compute(char_w: f32) -> Self {
        let group_gap = char_w;
        let addr_x = PAD_LEFT;
        let addr_w = ADDR_CHARS as f32 * char_w;
        let sep1_x = addr_x + addr_w + SECTION_GAP * 0.5;
        let hex_x = addr_x + addr_w + SECTION_GAP;
        let hex_w = HEX_CHARS as f32 * char_w;
        let sep2_x = hex_x + hex_w + SECTION_GAP * 0.5;
        let ascii_x = hex_x + hex_w + SECTION_GAP;
        let ascii_w = BYTES_PER_ROW as f32 * char_w;
        let total_w = ascii_x + ascii_w + PAD_LEFT;
        Self {
            addr_x,
            hex_x,
            ascii_x,
            total_w,
            sep1_x,
            sep2_x,
            char_w,
            group_gap,
        }
    }

    pub fn hex_byte_x(&self, col: usize) -> f32 {
        let base = col * 3;
        let group_offset = if col >= 8 { self.group_gap } else { 0.0 };
        self.hex_x + base as f32 * self.char_w + group_offset
    }

    pub fn ascii_byte_x(&self, col: usize) -> f32 {
        self.ascii_x + col as f32 * self.char_w
    }
}

/// 测量 monospace 字符宽度
pub(crate) fn measure_char_width(ui: &egui::Ui) -> f32 {
    let font = egui::FontId::monospace(FONT_SIZE);
    ui.painter()
        .layout_no_wrap("0".to_string(), font, egui::Color32::WHITE)
        .size()
        .x
}
