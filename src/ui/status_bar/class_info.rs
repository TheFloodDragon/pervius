//! class 文件版本信息状态栏 item
//!
//! @author sky

use super::item::{Alignment, ItemResponse, StatusItem};
use crate::shell::theme;
use eframe::egui;

/// 动态显示当前聚焦 .class 文件的版本信息
pub struct ClassInfoItem {
    text: Option<String>,
}

impl ClassInfoItem {
    pub fn new() -> Self {
        Self { text: None }
    }

    pub fn set_info(&mut self, info: Option<&str>) {
        match info {
            Some(s) => {
                if self.text.as_deref() != Some(s) {
                    self.text = Some(s.to_string());
                }
            }
            None => self.text = None,
        }
    }
}

impl StatusItem for ClassInfoItem {
    fn alignment(&self) -> Alignment {
        Alignment::Left
    }

    fn visible(&self) -> bool {
        self.text.is_some()
    }

    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse {
        let text = match &self.text {
            Some(t) => t,
            None => return ItemResponse { width: 0.0 },
        };
        let painter = ui.painter();
        let galley = painter.layout_no_wrap(
            text.clone(),
            egui::FontId::proportional(11.0),
            theme::TEXT_SECONDARY,
        );
        let w = galley.size().x;
        painter.galley(
            egui::pos2(x, center_y - galley.size().y / 2.0),
            galley,
            theme::TEXT_SECONDARY,
        );
        ItemResponse { width: w }
    }
}
