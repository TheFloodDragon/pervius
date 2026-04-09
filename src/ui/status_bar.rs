//! 状态栏（24px 高，窗口底部全宽）
//!
//! 左侧：版本号 | Java 版本
//! 中央：Decompiled / Bytecode / Hex 三视图切换
//! 右侧：反编译器 | 编码信息
//!
//! @author sky

use super::editor::view_toggle::ActiveView;
use crate::shell::theme;
use eframe::egui;

pub struct StatusBar;

impl StatusBar {
    /// 渲染状态栏，返回用户选择的新 ActiveView（若有聚焦 Tab）
    pub fn render(ui: &mut egui::Ui, active_view: Option<ActiveView>) -> Option<ActiveView> {
        let rect = ui.max_rect();
        // 背景 + 左侧文字 + 右侧文字先用 painter 画完
        {
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, theme::BG_DARKEST);
            let y = rect.center().y;
            let mut x = rect.left() + 12.0;
            x = Self::text_at(painter, x, y, "Pervius v0.1.0", theme::TEXT_MUTED);
            x = Self::separator(painter, x, y);
            Self::text_at(painter, x, y, "Java 17 (class 61.0)", theme::TEXT_SECONDARY);
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
        }
        // 中央视图切换（需要 ui 可变借用做交互）
        if let Some(view) = active_view {
            Some(Self::view_toggle(ui, rect, view))
        } else {
            None
        }
    }

    /// 在状态栏渲染三视图切换（右侧区域，紧挨右侧信息左边）
    fn view_toggle(ui: &mut egui::Ui, bar_rect: egui::Rect, active: ActiveView) -> ActiveView {
        let labels = ["Decompiled", "Bytecode", "Hex"];
        let views = [
            ActiveView::Decompiled,
            ActiveView::Bytecode,
            ActiveView::Hex,
        ];
        let font = egui::FontId::proportional(11.0);
        let painter = ui.painter();
        let pad = 6.0;
        let item_gap = 1.0;
        // 预计算各项文字宽度
        let text_widths: Vec<f32> = labels
            .iter()
            .map(|l| {
                painter
                    .layout_no_wrap(l.to_string(), font.clone(), theme::TEXT_PRIMARY)
                    .size()
                    .x
            })
            .collect();
        let item_widths: Vec<f32> = text_widths.iter().map(|w| w + pad * 2.0).collect();
        let container_pad = 2.0;
        let inner_w: f32 = item_widths.iter().sum::<f32>() + item_gap * (labels.len() as f32 - 1.0);
        let container_w = inner_w + container_pad * 2.0;
        let container_h = bar_rect.height() - 4.0;
        // 右侧信息宽度估算（UTF-8 | LF + sep + CFR 0.152 + margins）
        let right_info_w = Self::text_width(painter, "UTF-8  |  LF", 11.0)
            + 16.0
            + Self::text_width(painter, "CFR 0.152", 11.0)
            + 32.0;
        // 容器靠右，在右侧信息之前
        let container_right = bar_rect.right() - 12.0 - right_info_w;
        let container_rect = egui::Rect::from_min_size(
            egui::pos2(
                container_right - container_w,
                bar_rect.center().y - container_h / 2.0,
            ),
            egui::vec2(container_w, container_h),
        );
        painter.rect_filled(container_rect, 3.0, theme::BG_DARK);
        // 各项
        let mut result = active;
        let item_h = container_h - container_pad * 2.0;
        let mut ix = container_rect.left() + container_pad;
        let iy = container_rect.top() + container_pad;
        for (i, label) in labels.iter().enumerate() {
            let iw = item_widths[i];
            let item_rect = egui::Rect::from_min_size(egui::pos2(ix, iy), egui::vec2(iw, item_h));
            let is_active = active == views[i];
            let response = ui.interact(
                item_rect,
                ui.id().with(format!("vt_{i}")),
                egui::Sense::click(),
            );
            if response.clicked() {
                result = views[i];
            }
            if is_active {
                painter.rect_filled(item_rect, 2.0, theme::verdigris_alpha(40));
            }
            let color = if is_active {
                theme::VERDIGRIS
            } else if response.hovered() {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_MUTED
            };
            painter.text(
                item_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                font.clone(),
                color,
            );
            ix += iw + item_gap;
        }
        result
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
