//! 状态栏渲染（遍历注册的 items）
//!
//! @author sky

use super::item::{Alignment, StatusItem};
use crate::shell::theme;
use eframe::egui;

/// 状态栏
pub struct StatusBar {
    items: Vec<Box<dyn StatusItem>>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 注册一个 item
    pub fn add(&mut self, item: impl StatusItem + 'static) {
        self.items.push(Box::new(item));
    }

    /// 获取指定类型的 item 可变引用
    pub fn item_mut<T: StatusItem>(&mut self) -> Option<&mut T> {
        self.items
            .iter_mut()
            .find_map(|item| item.as_any_mut().downcast_mut::<T>())
    }

    /// 渲染状态栏
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::BG_DARK);
        let center_y = rect.center().y;
        let pad = 12.0;
        let sep_gap = 16.0;
        // 左侧 items 从左向右排列
        let mut left_x = rect.left() + pad;
        let mut left_first = true;
        for item in self.items.iter_mut() {
            if item.alignment() != Alignment::Left {
                continue;
            }
            if !left_first {
                left_x = Self::separator(ui, left_x, center_y, sep_gap);
            }
            let resp = item.render(ui, left_x, center_y);
            left_x += resp.width;
            left_first = false;
        }
        // 右侧 items 从右向左排列
        let mut right_x = rect.right() - pad;
        let mut right_first = true;
        for item in self.items.iter_mut().rev() {
            if item.alignment() != Alignment::Right {
                continue;
            }
            if !right_first {
                right_x = Self::separator_rev(ui, right_x, center_y, sep_gap);
            }
            let resp = item.render(ui, right_x, center_y);
            right_x -= resp.width;
            right_first = false;
        }
    }

    fn separator(ui: &egui::Ui, x: f32, y: f32, gap: f32) -> f32 {
        let sx = x + gap / 2.0;
        ui.painter().line_segment(
            [egui::pos2(sx, y - 7.0), egui::pos2(sx, y + 7.0)],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        x + gap
    }

    fn separator_rev(ui: &egui::Ui, x: f32, y: f32, gap: f32) -> f32 {
        let sx = x - gap / 2.0;
        ui.painter().line_segment(
            [egui::pos2(sx, y - 7.0), egui::pos2(sx, y + 7.0)],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        x - gap
    }
}
