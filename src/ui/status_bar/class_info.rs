//! class 文件版本信息状态栏 item
//!
//! @author sky

use crate::appearance::theme;
use egui_shell::components::panel::status_bar::{Alignment, ProgressItem};

egui_shell::define_progress_item! {
    /// 动态显示当前聚焦 .class 文件的版本信息
    pub struct ClassInfoItem;
}

impl ClassInfoItem {
    /// 创建 class 信息 item
    pub fn new() -> Self {
        Self::from_progress(ProgressItem::new(theme::TEXT_SECONDARY, Alignment::Left))
    }

    /// 设置版本信息，None 表示无 class 聚焦
    pub fn set_info(&mut self, info: Option<&str>) {
        match info {
            Some(s) => {
                if self.text() != s {
                    self.set_text(s);
                }
                self.set_visible(true);
            }
            None => self.set_visible(false),
        }
    }
}
