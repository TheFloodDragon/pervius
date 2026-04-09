//! 状态栏服务：自管理 items 注册、渲染、与编辑器的状态同步
//!
//! @author sky

use super::item::{Alignment, StatusItem};
use super::text_item::TextItem;
use super::view_toggle::ViewToggleItem;
use crate::shell::theme;
use crate::ui::editor::view_toggle::ActiveView;
use eframe::egui;

/// 状态栏服务
///
/// 内置默认 items（版本号、类信息、编码、反编译器版本、视图切换），
/// 外部只需调用 `render` 和 `sync` 即可。
pub struct StatusBar {
    items: Vec<Box<dyn StatusItem>>,
}

impl Default for StatusBar {
    fn default() -> Self {
        let mut s = Self { items: Vec::new() };
        s.add(TextItem::new(
            "Pervius v0.1.0",
            theme::TEXT_MUTED,
            Alignment::Left,
        ));
        s.add(
            TextItem::new(
                "Java 17 (class 61.0)",
                theme::TEXT_SECONDARY,
                Alignment::Left,
            )
            .context_only(),
        );
        s.add(TextItem::new("UTF-8  |  LF", theme::TEXT_MUTED, Alignment::Right).context_only());
        s.add(TextItem::new("CFR 0.152", theme::ACCENT_GREEN, Alignment::Right).context_only());
        s.add(ViewToggleItem::new());
        s
    }
}

impl StatusBar {
    /// 注册一个 item
    pub fn add(&mut self, item: impl StatusItem + 'static) {
        self.items.push(Box::new(item));
    }

    /// 获取指定类型的 item 可变引用
    pub fn item_mut<T: StatusItem>(&mut self) -> Option<&mut T> {
        self.items
            .iter_mut()
            .find_map(|item| item.downcast_mut::<T>())
    }

    /// 同步编辑器状态（ViewToggle + context_only items 可见性）
    pub fn sync_view(&mut self, active_view: Option<ActiveView>) {
        let has_tab = active_view.is_some();
        for item in &mut self.items {
            // 同步 context_only TextItem 的可见性
            if let Some(text) = item.downcast_mut::<TextItem>() {
                if text.is_context_only() {
                    text.set_visible(has_tab);
                }
            }
            // 同步 ViewToggle 的活跃视图
            if let Some(vt) = item.downcast_mut::<ViewToggleItem>() {
                vt.set_active(active_view);
            }
        }
    }

    /// 取出用户通过 ViewToggle 切换的新视图（渲染后调用）
    pub fn take_view_change(&mut self) -> Option<ActiveView> {
        self.item_mut::<ViewToggleItem>()?.take_changed()
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
            if item.alignment() != Alignment::Left || !item.visible() {
                continue;
            }
            if !left_first {
                Self::paint_separator(ui, left_x + sep_gap / 2.0, center_y);
                left_x += sep_gap;
            }
            let resp = item.render(ui, left_x, center_y);
            left_x += resp.width;
            left_first = false;
        }
        // 右侧 items 从右向左排列
        let mut right_x = rect.right() - pad;
        let mut right_first = true;
        for item in self.items.iter_mut().rev() {
            if item.alignment() != Alignment::Right || !item.visible() {
                continue;
            }
            if !right_first {
                Self::paint_separator(ui, right_x - sep_gap / 2.0, center_y);
                right_x -= sep_gap;
            }
            let resp = item.render(ui, right_x, center_y);
            right_x -= resp.width;
            right_first = false;
        }
    }

    fn paint_separator(ui: &egui::Ui, x: f32, y: f32) {
        ui.painter().line_segment(
            [egui::pos2(x, y - 7.0), egui::pos2(x, y + 7.0)],
            egui::Stroke::new(1.0, theme::BORDER),
        );
    }
}
