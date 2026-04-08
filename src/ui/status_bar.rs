//! 状态栏（24px 高，窗口底部全宽）
//!
//! 对照 status_bar.slint：版本号 | Java 版本 | 反编译器 | 编码信息。
//! 目前为静态占位，后续接入真实数据。
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;

pub struct StatusBar;

impl StatusBar {
    pub fn render(ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, theme::BG_DARKEST);
        let y = rect.center().y;
        let mut x = rect.left() + 12.0;
        // 版本号
        x = Self::text_at(painter, x, y, "Pervius v0.1.0", theme::TEXT_MUTED);
        x = Self::separator(painter, x, y);
        // Java 版本
        x = Self::text_at(painter, x, y, "Java 17 (class 61.0)", theme::TEXT_SECONDARY);
        // 右侧信息：从右往左画
        let right = rect.right() - 12.0;
        let r = Self::text_width(painter, "UTF-8  |  LF", 11.0);
        painter.text(
            egui::pos2(right - r, y),
            egui::Align2::LEFT_CENTER,
            "UTF-8  |  LF",
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
        let r2 = Self::text_width(painter, "CFR 0.152", 11.0);
        let sep_x = right - r - 16.0;
        Self::separator(painter, sep_x - 8.0, y);
        painter.text(
            egui::pos2(sep_x - 8.0 - r2 - 8.0, y),
            egui::Align2::LEFT_CENTER,
            "CFR 0.152",
            egui::FontId::proportional(11.0),
            theme::ACCENT_GREEN,
        );
        let _ = x;
    }

    fn text_at(painter: &egui::Painter, x: f32, y: f32, text: &str, color: egui::Color32) -> f32 {
        let font = egui::FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), color);
        let w = galley.size().x;
        painter.galley(egui::pos2(x, y - galley.size().y / 2.0), galley, color);
        x + w
    }

    fn separator(painter: &egui::Painter, x: f32, y: f32) -> f32 {
        let sx = x + 8.0;
        painter.line_segment(
            [egui::pos2(sx, y - 7.0), egui::pos2(sx, y + 7.0)],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        sx + 9.0
    }

    fn text_width(painter: &egui::Painter, text: &str, size: f32) -> f32 {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(size),
            egui::Color32::TRANSPARENT,
        );
        galley.size().x
    }
}
