//! 状态栏通用组件：item trait、布局引擎、分隔符绘制
//!
//! 颜色通过 `StatusBarTheme` 注入，不硬编码任何色板。
//!
//! @author sky

use eframe::egui;
use std::any::Any;

/// 状态栏项目的位置
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Right,
}

/// 状态栏项目渲染结果
pub struct ItemResponse {
    /// 该 item 占用的宽度
    pub width: f32,
}

/// 状态栏可注册项目
///
/// 每个 item 负责在给定的 x 坐标处渲染自身，返回占用宽度。
/// 左侧 item 从左向右排列，右侧 item 从右向左排列。
pub trait StatusItem: Any {
    fn alignment(&self) -> Alignment;
    /// 当前是否可见（不可见时跳过渲染且不占空间）
    fn visible(&self) -> bool {
        true
    }
    /// 渲染 item 并返回占用宽度
    ///
    /// - `ui`: 状态栏 UI（用于交互和绘制）
    /// - `x`: 起始 x 坐标（左侧 item 为左边缘，右侧 item 为右边缘）
    /// - `center_y`: 状态栏垂直中心
    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse;
}

/// blanket impl：所有 StatusItem 实现者自动获得 downcast 能力
impl dyn StatusItem {
    pub fn downcast_mut<T: StatusItem>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }
}

/// 状态栏配色
#[derive(Clone)]
pub struct StatusBarTheme {
    /// 背景填充色
    pub bg: egui::Color32,
    /// 分隔符颜色
    pub separator: egui::Color32,
}

/// 通用状态栏容器
///
/// 管理一组 `StatusItem`，按 Alignment 分组渲染，自动插入分隔符。
pub struct StatusBarWidget {
    items: Vec<Box<dyn StatusItem>>,
    theme: StatusBarTheme,
}

impl StatusBarWidget {
    pub fn new(theme: StatusBarTheme) -> Self {
        Self {
            items: Vec::new(),
            theme,
        }
    }

    /// 注册一个 item
    pub fn add(&mut self, item: impl StatusItem + 'static) {
        self.items.push(Box::new(item));
    }

    /// 获取所有 items 的可变引用切片（供业务层遍历 downcast）
    pub fn items_mut(&mut self) -> &mut [Box<dyn StatusItem>] {
        &mut self.items
    }

    /// 获取指定类型的 item 可变引用
    pub fn item_mut<T: StatusItem>(&mut self) -> Option<&mut T> {
        self.items
            .iter_mut()
            .find_map(|item| item.downcast_mut::<T>())
    }

    /// 渲染状态栏
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, self.theme.bg);
        let center_y = rect.center().y;
        let pad = 12.0;
        let sep_gap = 16.0;
        let sep_color = self.theme.separator;
        // 左侧 items 从左向右排列
        let mut left_x = rect.left() + pad;
        let mut left_first = true;
        for item in self.items.iter_mut() {
            if item.alignment() != Alignment::Left || !item.visible() {
                continue;
            }
            if !left_first {
                Self::paint_separator(ui, left_x + sep_gap / 2.0, center_y, sep_color);
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
                Self::paint_separator(ui, right_x - sep_gap / 2.0, center_y, sep_color);
                right_x -= sep_gap;
            }
            let resp = item.render(ui, right_x, center_y);
            right_x -= resp.width;
            right_first = false;
        }
    }

    fn paint_separator(ui: &egui::Ui, x: f32, y: f32, color: egui::Color32) {
        ui.painter().line_segment(
            [egui::pos2(x, y - 7.0), egui::pos2(x, y + 7.0)],
            egui::Stroke::new(1.0, color),
        );
    }
}
