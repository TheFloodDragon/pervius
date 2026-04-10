//! 跨模块复用的通用 widget
//!
//! @author sky

use crate::appearance::theme;
use eframe::egui;

/// 扁平文本按钮（菜单栏、搜索分类、模式切换等通用小型按钮）
///
/// 基础样式：transparent fill、corner_radius 3、proportional 字体。
/// 可选 active 状态：active 时 VERDIGRIS 文字 + BG_HOVER 底色。
pub struct FlatButton<'a> {
    label: &'a str,
    font_size: f32,
    font_family: Option<egui::FontFamily>,
    active: Option<bool>,
    /// inactive 时的文字颜色（默认 TEXT_SECONDARY）
    inactive_color: egui::Color32,
    min_size: egui::Vec2,
}

impl<'a> FlatButton<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            font_size: 12.0,
            font_family: None,
            active: None,
            inactive_color: theme::TEXT_SECONDARY,
            min_size: egui::vec2(0.0, 24.0),
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn font_family(mut self, family: egui::FontFamily) -> Self {
        self.font_family = Some(family);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn inactive_color(mut self, color: egui::Color32) -> Self {
        self.inactive_color = color;
        self
    }

    pub fn min_size(mut self, size: egui::Vec2) -> Self {
        self.min_size = size;
        self
    }
}

impl egui::Widget for FlatButton<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let is_active = matches!(self.active, Some(true));
        let color = match self.active {
            Some(true) => theme::VERDIGRIS,
            Some(false) => self.inactive_color,
            None => theme::TEXT_PRIMARY,
        };
        let base_fill = if is_active {
            theme::BG_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };
        // 通过 widget visuals 控制三态 fill，不用 Button::fill（会覆盖所有状态）
        ui.scope(|ui| {
            let vis = &mut ui.style_mut().visuals;
            // 禁用 focus ring（默认灰白色在深色主题下刺眼）
            vis.selection.stroke = egui::Stroke::NONE;
            let wv = &mut vis.widgets;
            wv.inactive.weak_bg_fill = base_fill;
            wv.inactive.bg_stroke = egui::Stroke::NONE;
            wv.hovered.weak_bg_fill = theme::BG_HOVER;
            wv.hovered.bg_stroke = egui::Stroke::NONE;
            wv.active.weak_bg_fill = theme::BG_LIGHT;
            wv.active.bg_stroke = egui::Stroke::NONE;
            let text = if let Some(ref family) = self.font_family {
                egui::RichText::new(self.label)
                    .font(egui::FontId::new(self.font_size, family.clone()))
                    .color(color)
            } else {
                egui::RichText::new(self.label)
                    .size(self.font_size)
                    .color(color)
            };
            ui.add(
                egui::Button::new(text)
                    .corner_radius(3)
                    .min_size(self.min_size),
            )
        })
        .inner
    }
}
