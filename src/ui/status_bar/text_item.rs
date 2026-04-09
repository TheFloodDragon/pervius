//! 纯文本状态栏 item
//!
//! @author sky

use super::item::{Alignment, ItemResponse, StatusItem};
use eframe::egui;
use std::any::Any;

/// 纯文本 item（无交互）
pub struct TextItem {
    text: String,
    color: egui::Color32,
    alignment: Alignment,
}

impl TextItem {
    pub fn new(text: impl Into<String>, color: egui::Color32, alignment: Alignment) -> Self {
        Self {
            text: text.into(),
            color,
            alignment,
        }
    }
}

impl StatusItem for TextItem {
    fn alignment(&self) -> Alignment {
        self.alignment
    }

    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse {
        let painter = ui.painter();
        let font = egui::FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(self.text.clone(), font, self.color);
        let w = galley.size().x;
        match self.alignment {
            Alignment::Left => {
                painter.galley(
                    egui::pos2(x, center_y - galley.size().y / 2.0),
                    galley,
                    self.color,
                );
            }
            Alignment::Right => {
                painter.galley(
                    egui::pos2(x - w, center_y - galley.size().y / 2.0),
                    galley,
                    self.color,
                );
            }
        }
        ItemResponse { width: w }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
