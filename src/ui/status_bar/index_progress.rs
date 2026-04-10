//! 搜索索引构建进度状态栏 item
//!
//! @author sky

use crate::appearance::theme;
use egui_shell::components::panel::status_bar::{Alignment, ProgressItem};
use rust_i18n::t;

egui_shell::define_progress_item! {
    /// 搜索索引构建进度 item（左侧显示，构建期间可见）
    pub struct IndexProgressItem;
}

impl IndexProgressItem {
    /// 创建索引进度 item
    pub fn new() -> Self {
        Self::from_progress(ProgressItem::new(theme::TEXT_MUTED, Alignment::Left))
    }

    /// 更新索引构建进度，None 表示无任务
    pub fn set_progress(&mut self, info: Option<(u32, u32)>) {
        match info {
            Some((current, total)) => {
                self.set_text(
                    t!("status.indexing_progress", current = current, total = total).to_string(),
                );
                self.set_visible(true);
            }
            None => {
                self.set_visible(false);
            }
        }
    }
}
