//! 扁平文本按钮（菜单栏、搜索分类、模式切换等通用小型按钮）
//!
//! @author sky

use eframe::egui;

/// 扁平按钮配色
#[derive(Clone)]
pub struct FlatButtonTheme {
    /// 无 active 状态时的文字颜色
    pub text_primary: egui::Color32,
    /// active = true 时的文字颜色
    pub text_active: egui::Color32,
    /// active = false 时的默认文字颜色
    pub text_inactive: egui::Color32,
    /// active = true 时的底色 / hover 态底色
    pub bg_hover: egui::Color32,
    /// pressed 态底色
    pub bg_pressed: egui::Color32,
}

tabookit::class! {
    /// 扁平文本按钮
    ///
    /// 基础样式：transparent fill、corner_radius 3、proportional 字体。
    /// 可选 active 状态：active 时使用 `text_active` 文字色 + `bg_hover` 底色。
    pub struct FlatButton<'a> {
        /// 按钮文字
        label: &'a str,
        /// 字体大小（默认 12.0）
        font_size: f32,
        /// 字体族（默认 proportional，可设为 codicon 等图标字体）
        font_family: Option<egui::FontFamily>,
        /// 激活状态：None = 无状态按钮，Some(true) = 激活，Some(false) = 未激活
        active: Option<bool>,
        /// inactive 时的文字颜色（默认取 theme.text_inactive）
        inactive_color: egui::Color32,
        /// 最小尺寸（默认 0x24）
        min_size: egui::Vec2,
        /// 配色主题
        theme: &'a FlatButtonTheme,
    }

    pub fn new(label: &'a str, theme: &'a FlatButtonTheme) -> Self {
        Self {
            label,
            font_size: 12.0,
            font_family: None,
            active: None,
            inactive_color: theme.text_inactive,
            min_size: egui::vec2(0.0, 24.0),
            theme,
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
            Some(true) => self.theme.text_active,
            Some(false) => self.inactive_color,
            None => self.theme.text_primary,
        };
        let base_fill = if is_active {
            self.theme.bg_hover
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
            wv.hovered.weak_bg_fill = self.theme.bg_hover;
            wv.hovered.bg_stroke = egui::Stroke::NONE;
            wv.active.weak_bg_fill = self.theme.bg_pressed;
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
