//! 反编译进度状态栏 item
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;
use egui_shell::components::status_bar::{Alignment, ItemResponse, StatusItem};
use rust_i18n::t;

/// 反编译进度 item（左侧显示，反编译期间可见）
pub struct DecompileProgressItem {
    text: String,
    visible: bool,
}

impl DecompileProgressItem {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            visible: false,
        }
    }

    /// 设置单文件反编译状态
    pub fn set_single(&mut self, name: &str) {
        self.text = t!("status.decompiling_single", name = name).to_string();
        self.visible = true;
    }

    /// 更新批量反编译进度，None 表示无任务
    pub fn set_progress(&mut self, info: Option<(&str, u32, u32)>) {
        match info {
            Some((name, current, total)) => {
                if total == 0 {
                    self.text = t!("status.decompiling_init", name = name, count = 0).to_string();
                } else {
                    self.text = t!(
                        "status.decompiling_progress",
                        name = name,
                        current = current,
                        total = total
                    )
                    .to_string();
                }
                self.visible = true;
            }
            None => {
                self.visible = false;
            }
        }
    }
}

impl StatusItem for DecompileProgressItem {
    fn alignment(&self) -> Alignment {
        Alignment::Left
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse {
        let painter = ui.painter();
        let galley = painter.layout_no_wrap(
            self.text.clone(),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
        let w = galley.size().x;
        painter.galley(
            egui::pos2(x, center_y - galley.size().y / 2.0),
            galley,
            theme::TEXT_MUTED,
        );
        ItemResponse { width: w }
    }
}
