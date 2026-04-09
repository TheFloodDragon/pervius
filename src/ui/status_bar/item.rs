//! StatusBar item trait
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
