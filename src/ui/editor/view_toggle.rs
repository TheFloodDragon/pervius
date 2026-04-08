//! Decompiled / Bytecode / Hex 三视图切换栏
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;

/// 活跃视图
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Decompiled,
    Bytecode,
    Hex,
}

/// 渲染视图切换工具栏，返回新的 active_view
pub fn render(ui: &mut egui::Ui, rect: egui::Rect, active: ActiveView) -> ActiveView {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::BG_DARK);
    let container_h = 26.0;
    let container_y = rect.center().y - container_h / 2.0;
    let labels = ["Decompiled", "Bytecode", "Hex"];
    let views = [
        ActiveView::Decompiled,
        ActiveView::Bytecode,
        ActiveView::Hex,
    ];
    let font = egui::FontId::proportional(11.0);
    // 预计算各按钮宽度
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| {
            painter
                .layout_no_wrap(l.to_string(), font.clone(), theme::TEXT_PRIMARY)
                .size()
                .x
                + 16.0
        })
        .collect();
    let total_w: f32 = widths.iter().sum::<f32>() + 4.0 * 2.0 + 2.0 * (labels.len() as f32 - 1.0);
    // 容器
    let container_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 12.0, container_y),
        egui::vec2(total_w + 8.0, container_h),
    );
    painter.rect_filled(container_rect, 6.0, theme::BG_MEDIUM);
    painter.rect_stroke(
        container_rect,
        6.0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Middle,
    );
    // 按钮
    let mut result = active;
    let mut bx = container_rect.left() + 4.0;
    for (i, label) in labels.iter().enumerate() {
        let bw = widths[i];
        let btn_rect =
            egui::Rect::from_min_size(egui::pos2(bx, container_y + 2.0), egui::vec2(bw, 22.0));
        let is_active = active == views[i];
        let response = ui.interact(
            btn_rect,
            ui.id().with(format!("view_toggle_{i}")),
            egui::Sense::click(),
        );
        if response.clicked() {
            result = views[i];
        }
        if is_active {
            painter.rect_filled(btn_rect, 4.0, theme::verdigris_alpha(64));
        }
        let color = if is_active {
            theme::VERDIGRIS
        } else {
            theme::TEXT_SECONDARY
        };
        painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            color,
        );
        bx += bw + 2.0;
    }
    result
}
