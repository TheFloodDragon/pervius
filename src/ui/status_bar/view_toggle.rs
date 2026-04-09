//! 视图切换状态栏 item（Decompiled / Bytecode / Hex）
//!
//! @author sky

use super::item::{Alignment, ItemResponse, StatusItem};
use crate::shell::theme;
use crate::ui::editor::view_toggle::ActiveView;
use eframe::egui;

/// 三视图切换 item
pub struct ViewToggleItem {
    /// 当前活跃视图（None 表示无 tab 聚焦，不显示）
    active: Option<ActiveView>,
    /// 用户点击后的新视图
    changed: Option<ActiveView>,
}

const TOGGLE_OPTIONS: [(&str, ActiveView); 3] = [
    ("Decompiled", ActiveView::Decompiled),
    ("Bytecode", ActiveView::Bytecode),
    ("Hex", ActiveView::Hex),
];

impl ViewToggleItem {
    pub fn new() -> Self {
        Self {
            active: None,
            changed: None,
        }
    }

    /// 外部设置当前视图
    pub fn set_active(&mut self, view: Option<ActiveView>) {
        self.active = view;
    }

    /// 取出用户选择的新视图（取后清空）
    pub fn take_changed(&mut self) -> Option<ActiveView> {
        self.changed.take()
    }
}

impl StatusItem for ViewToggleItem {
    fn alignment(&self) -> Alignment {
        Alignment::Right
    }

    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse {
        let active = match self.active {
            Some(v) => v,
            None => return ItemResponse { width: 0.0 },
        };
        let painter = ui.painter();
        let font = egui::FontId::proportional(11.0);
        let pad = 6.0;
        let item_gap = 1.0;
        let container_pad = 2.0;
        // 预计算每个选项的文字宽度和 item 宽度
        let item_widths: Vec<f32> = TOGGLE_OPTIONS
            .iter()
            .map(|(label, _)| {
                painter
                    .layout_no_wrap(label.to_string(), font.clone(), theme::TEXT_PRIMARY)
                    .size()
                    .x
                    + pad * 2.0
            })
            .collect();
        let inner_w: f32 =
            item_widths.iter().sum::<f32>() + item_gap * (TOGGLE_OPTIONS.len() as f32 - 1.0);
        let container_w = inner_w + container_pad * 2.0;
        let bar_height = theme::STATUS_BAR_HEIGHT;
        let container_h = bar_height - 4.0;
        // 右对齐：容器右边缘在 x 处
        let container_rect = egui::Rect::from_min_size(
            egui::pos2(x - container_w, center_y - container_h / 2.0),
            egui::vec2(container_w, container_h),
        );
        painter.rect_filled(container_rect, 3.0, theme::BG_DARKEST);
        let item_h = container_h - container_pad * 2.0;
        let mut ix = container_rect.left() + container_pad;
        let iy = container_rect.top() + container_pad;
        for (i, (label, view)) in TOGGLE_OPTIONS.iter().enumerate() {
            let iw = item_widths[i];
            let item_rect = egui::Rect::from_min_size(egui::pos2(ix, iy), egui::vec2(iw, item_h));
            let is_active = active == *view;
            let response = ui.interact(
                item_rect,
                ui.id().with(format!("vt_{i}")),
                egui::Sense::click(),
            );
            if response.clicked() {
                self.changed = Some(*view);
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
        ItemResponse { width: container_w }
    }
}
