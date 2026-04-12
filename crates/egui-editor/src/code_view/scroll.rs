//! 代码视图滚动辅助
//!
//! @author sky

use eframe::egui;

/// 检测拖拽选择时的边缘滚动和滚轮事件
pub(crate) fn detect_edge_scroll(
    response: &egui::Response,
    ui: &egui::Ui,
) -> (egui::Vec2, egui::Vec2) {
    let clip = ui.clip_rect();
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let edge_zone = 30.0;
    let max_speed = 800.0;
    let axis_speed = |pos: f32, min: f32, max: f32| {
        if pos < min {
            let dist = min - pos + edge_zone;
            (dist / edge_zone).min(3.0) * max_speed
        } else if pos > max {
            let dist = pos - max + edge_zone;
            -((dist / edge_zone).min(3.0) * max_speed)
        } else if pos < min + edge_zone {
            let factor = (min + edge_zone - pos) / edge_zone;
            factor * max_speed * 0.3
        } else if pos > max - edge_zone {
            let factor = (pos - max + edge_zone) / edge_zone;
            -(factor * max_speed * 0.3)
        } else {
            0.0
        }
    };
    let mut edge_delta = egui::Vec2::ZERO;
    // 鼠标靠近或超出视口边缘时自动滚动
    if let Some(pos) = response.interact_pointer_pos() {
        edge_delta = egui::vec2(
            axis_speed(pos.x, clip.left(), clip.right()) * dt,
            axis_speed(pos.y, clip.top(), clip.bottom()) * dt,
        );
    }
    let wheel = ui.input(|i| i.smooth_scroll_delta);
    (edge_delta, wheel)
}

/// 应用滚动偏移（确保 scroll_with_delta 被正确的 ScrollArea 消费）
pub(crate) fn apply_scroll_delta(
    ui: &mut egui::Ui,
    edge_delta: egui::Vec2,
    wheel_delta: egui::Vec2,
) {
    if edge_delta != egui::Vec2::ZERO {
        ui.scroll_with_delta_animation(edge_delta, egui::style::ScrollAnimation::none());
        ui.ctx().request_repaint();
    }
    if wheel_delta != egui::Vec2::ZERO {
        ui.scroll_with_delta_animation(wheel_delta, egui::style::ScrollAnimation::none());
        ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    }
}
