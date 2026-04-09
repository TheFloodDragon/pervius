//! 纯文本状态栏 item
//!
//! @author sky

use super::item::{Alignment, ItemResponse, StatusItem};
use eframe::egui;

/// 纯文本 item（无交互）
pub struct TextItem {
    text: String,
    color: egui::Color32,
    alignment: Alignment,
    /// 仅在有活跃 tab 时显示（由 StatusBar 统一控制）
    context_only: bool,
    visible: bool,
}

impl TextItem {
    pub fn new(text: impl Into<String>, color: egui::Color32, alignment: Alignment) -> Self {
        Self {
            text: text.into(),
            color,
            alignment,
            context_only: false,
            visible: true,
        }
    }

    /// 标记为文件上下文相关（无活跃 tab 时自动隐藏）
    pub fn context_only(mut self) -> Self {
        self.context_only = true;
        self
    }

    pub fn is_context_only(&self) -> bool {
        self.context_only
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl StatusItem for TextItem {
    fn alignment(&self) -> Alignment {
        self.alignment
    }

    fn visible(&self) -> bool {
        self.visible
    }

    fn render(&mut self, ui: &mut egui::Ui, x: f32, center_y: f32) -> ItemResponse {
        let painter = ui.painter();
        let galley = painter.layout_no_wrap(
            self.text.clone(),
            egui::FontId::proportional(11.0),
            self.color,
        );
        let w = galley.size().x;
        let draw_x = match self.alignment {
            Alignment::Left => x,
            Alignment::Right => x - w,
        };
        painter.galley(
            egui::pos2(draw_x, center_y - galley.size().y / 2.0),
            galley,
            self.color,
        );
        ItemResponse { width: w }
    }
}
