//! 四边 + 四角 resize 交互 + 右下角 grip 指示线
//!
//! 8 个 resize zone（右/下/左/上 + 四角），每个 zone 通过布尔标记
//! 哪些边参与尺寸变化。左/上边拖拽时同步移动窗口位置，保持对边不动。
//!
//! @author sky

use super::{FloatingWindow, GRAB};
use crate::WindowTheme;
use eframe::egui;

/// resize zone 描述：抓取区域 + 参与变化的边 + 光标图标
struct Zone {
    rect: egui::Rect,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
    cursor: egui::CursorIcon,
}

/// 根据窗口 rect 构建 8 个 resize zone
fn build_zones(r: egui::Rect) -> [Zone; 8] {
    let g = GRAB;
    [
        // 四边
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.right() - g..=r.right() + g,
                r.top() + g..=r.bottom() - g,
            ),
            left: false,
            top: false,
            right: true,
            bottom: false,
            cursor: egui::CursorIcon::ResizeHorizontal,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.left() + g..=r.right() - g,
                r.bottom() - g..=r.bottom() + g,
            ),
            left: false,
            top: false,
            right: false,
            bottom: true,
            cursor: egui::CursorIcon::ResizeVertical,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.left() - g..=r.left() + g,
                r.top() + g..=r.bottom() - g,
            ),
            left: true,
            top: false,
            right: false,
            bottom: false,
            cursor: egui::CursorIcon::ResizeHorizontal,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.left() + g..=r.right() - g,
                r.top() - g..=r.top() + g,
            ),
            left: false,
            top: true,
            right: false,
            bottom: false,
            cursor: egui::CursorIcon::ResizeVertical,
        },
        // 四角
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.right() - g..=r.right() + g,
                r.bottom() - g..=r.bottom() + g,
            ),
            left: false,
            top: false,
            right: true,
            bottom: true,
            cursor: egui::CursorIcon::ResizeNwSe,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.left() - g..=r.left() + g,
                r.top() - g..=r.top() + g,
            ),
            left: true,
            top: true,
            right: false,
            bottom: false,
            cursor: egui::CursorIcon::ResizeNwSe,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.right() - g..=r.right() + g,
                r.top() - g..=r.top() + g,
            ),
            left: false,
            top: true,
            right: true,
            bottom: false,
            cursor: egui::CursorIcon::ResizeNeSw,
        },
        Zone {
            rect: egui::Rect::from_x_y_ranges(
                r.left() - g..=r.left() + g,
                r.bottom() - g..=r.bottom() + g,
            ),
            left: true,
            top: false,
            right: false,
            bottom: true,
            cursor: egui::CursorIcon::ResizeNeSw,
        },
    ]
}

impl FloatingWindow {
    /// 处理 8 个 resize zone 的拖拽交互
    pub(super) fn handle_resize(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        theme: &WindowTheme,
    ) {
        let size = self.size.unwrap_or(self.default_size);
        let min = self.min_size;
        let zones = build_zones(rect);
        let mut new_size = size;
        let mut pos_delta = egui::Vec2::ZERO;
        let mut any_active = false;
        for (i, zone) in zones.iter().enumerate() {
            let resp = ui.interact(zone.rect, self.id.with("rz").with(i), egui::Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(zone.cursor);
            }
            if !resp.dragged() {
                continue;
            }
            any_active = true;
            let d = resp.drag_delta();
            // 右/下边拖拽：直接增减尺寸
            if zone.right {
                new_size.x += d.x;
            }
            if zone.bottom {
                new_size.y += d.y;
            }
            // 左/上边拖拽：反向调整尺寸并移动窗口位置，保持对边不动
            if zone.left {
                let w = (size.x - d.x).max(min.x);
                pos_delta.x += size.x - w;
                new_size.x = w;
            }
            if zone.top {
                let h = (size.y - d.y).max(min.y);
                pos_delta.y += size.y - h;
                new_size.y = h;
            }
        }
        if any_active {
            new_size.x = new_size.x.max(min.x);
            new_size.y = new_size.y.max(min.y);
            self.size = Some(new_size);
            if let Some(pos) = &mut self.pos {
                *pos += pos_delta;
            }
            ui.ctx().request_repaint();
        }
        paint_resize_grip(ui.painter(), rect, theme.text_muted);
    }
}

/// 右下角三条斜线 grip 指示
fn paint_resize_grip(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = egui::Color32::from_white_alpha(60).lerp_to_gamma(color, 0.5);
    let stroke = egui::Stroke::new(1.0, c);
    let br = rect.right_bottom();
    for i in 1..=3 {
        let offset = i as f32 * 3.5;
        painter.line_segment(
            [
                egui::pos2(br.x - offset, br.y - 1.5),
                egui::pos2(br.x - 1.5, br.y - offset),
            ],
            stroke,
        );
    }
}
