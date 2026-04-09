//! Island 圆角背景绘制
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;

/// 绘制 island 圆角背景（深色，与窗口底色 BG_DARK 形成对比）
pub fn paint(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(theme::ISLAND_RADIUS),
        theme::BG_DARKEST,
    );
}

/// 在 island 四角绘制窗口底色遮罩，裁剪溢出的方角内容
///
/// 每个角是一个 r×r 正方形，内部挖去四分之一圆弧，剩余区域填充窗口底色。
/// 通过 mesh 三角扇形实现：圆心 + 圆弧上若干采样点 + 两条直边端点。
pub fn paint_corner_mask(ui: &egui::Ui, rect: egui::Rect) {
    let r = theme::ISLAND_RADIUS as f32;
    let color = theme::BG_DARK;
    let painter = ui.painter();
    // 四个角：(角落坐标, 圆心坐标, 起始角度)
    let corners = [
        (
            rect.left_top(),
            egui::pos2(rect.left() + r, rect.top() + r),
            std::f32::consts::PI,
        ),
        (
            egui::pos2(rect.right(), rect.top()),
            egui::pos2(rect.right() - r, rect.top() + r),
            -std::f32::consts::FRAC_PI_2,
        ),
        (
            egui::pos2(rect.right(), rect.bottom()),
            egui::pos2(rect.right() - r, rect.bottom() - r),
            0.0,
        ),
        (
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.left() + r, rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
        ),
    ];
    let segments = 8;
    let quarter = std::f32::consts::FRAC_PI_2;
    for (corner, center, start_angle) in &corners {
        let mut mesh = egui::Mesh::default();
        let corner_idx = mesh.vertices.len() as u32;
        mesh.colored_vertex(*corner, color);
        for i in 0..=segments {
            let t = *start_angle + quarter * (i as f32 / segments as f32);
            let p = egui::pos2(center.x + r * t.cos(), center.y + r * t.sin());
            mesh.colored_vertex(p, color);
        }
        for i in 0..segments {
            let a = corner_idx + 1 + i as u32;
            mesh.add_triangle(corner_idx, a, a + 1);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}
