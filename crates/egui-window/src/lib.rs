//! 主题化浮动窗口：Area + Frame 自绘，内置 pin/unpin 逻辑
//!
//! 不使用 egui::Window（其边缘 resize 存在帧同步缺陷），
//! 改为 egui::Area + egui::Frame 手动控制窗口尺寸和位置。
//! Area 设为 movable(false)，移动和 resize 完全由本层代码控制，互不冲突。
//!
//! @author sky

use eframe::egui;

/// 浮动窗口主题配色
#[derive(Clone)]
pub struct WindowTheme {
    /// 窗口外框样式（fill / stroke / corner_radius / shadow）
    pub frame: egui::Frame,
    /// 自定义 header 高度
    pub header_height: f32,
    /// 强调色（图标、active pin）
    pub accent: egui::Color32,
    /// 主文字色（标题）
    pub text_primary: egui::Color32,
    /// 暗淡文字色（inactive pin）
    pub text_muted: egui::Color32,
    /// 按钮 active 底色（pin 激活时）
    pub bg_active: egui::Color32,
    /// widget hover 底色
    pub bg_hover: egui::Color32,
    /// widget pressed 底色
    pub bg_pressed: egui::Color32,
    /// 分隔线颜色
    pub separator: egui::Color32,
    /// 图标字体族（用于 header icon + pin icon）
    pub icon_font: egui::FontFamily,
    /// Pin 图标字符
    pub pin_icon: char,
}

/// 边缘 resize 抓取宽度
const GRAB: f32 = 5.0;

/// 带 pin 支持的主题化浮动窗口
///
/// 基于 `egui::Area`（movable=false）+ `egui::Frame` 自绘。
/// 移动通过 header 拖拽实现，resize 通过四边/四角手柄实现，互不冲突。
pub struct FloatingWindow {
    id: egui::Id,
    title: String,
    icon: Option<char>,
    default_size: egui::Vec2,
    min_size: egui::Vec2,
    resizable: bool,
    pub open: bool,
    pub pinned: bool,
    /// 当前窗口尺寸
    size: Option<egui::Vec2>,
    /// 当前窗口位置（左上角），None 表示首次打开居中
    pos: Option<egui::Pos2>,
    /// header 右侧按钮区域的左边界 x 坐标（屏幕坐标），drag 区域会排除此右侧
    header_right_x: f32,
}

impl Default for FloatingWindow {
    fn default() -> Self {
        Self {
            id: egui::Id::NULL,
            title: String::new(),
            icon: None,
            default_size: egui::vec2(600.0, 400.0),
            min_size: egui::vec2(300.0, 200.0),
            resizable: true,
            open: false,
            pinned: false,
            size: None,
            pos: None,
            header_right_x: f32::MAX,
        }
    }
}

impl FloatingWindow {
    pub fn new(id: impl Into<egui::Id>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn default_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.default_size = size.into();
        self
    }

    pub fn min_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.min_size = size.into();
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn open(&mut self) {
        self.open = true;
        self.pos = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pinned = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 渲染浮动窗口
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &WindowTheme,
        header_right: impl FnOnce(&mut egui::Ui),
        content: impl FnOnce(&mut egui::Ui),
    ) {
        if !self.open {
            return;
        }
        if !self.pinned && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close();
            return;
        }
        let size = self.size.unwrap_or(self.default_size);
        let screen = ctx.screen_rect();
        let pos = self.pos.unwrap_or_else(|| {
            egui::pos2(
                screen.center().x - size.x * 0.5,
                screen.center().y - size.y * 0.5,
            )
        });
        // 记住位置（首次打开后不再重算）
        self.pos = Some(pos);
        let area_resp = egui::Area::new(self.id)
            .movable(false)
            .sense(egui::Sense::hover())
            .current_pos(pos)
            .constrain(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let frame_resp = theme.frame.show(ui, |ui| {
                    let origin = ui.available_rect_before_wrap().min;
                    let rect = egui::Rect::from_min_size(origin, size);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .id(self.id.with("content"))
                            .max_rect(rect),
                    );
                    child.set_clip_rect(rect);
                    apply_style(&mut child, theme);
                    self.render_header(&mut child, theme, header_right);
                    separator(&mut child, theme.separator);
                    content(&mut child);
                    ui.allocate_rect(rect, egui::Sense::hover());
                });
                let frame_rect = frame_resp.response.rect;
                // header 拖拽移动
                self.handle_move(ui, frame_rect, theme);
                // 四边四角 resize
                if self.resizable {
                    self.handle_resize(ui, frame_rect, theme);
                }
            });
        // 未 pin 时点击窗口外部关闭
        if !self.pinned {
            let window_rect = area_resp.response.rect.expand(4.0);
            let clicked_outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !window_rect.contains(p))
            });
            if clicked_outside {
                self.close();
            }
        }
    }

    /// header 区域拖拽移动窗口（排除右侧按钮区域，避免吞掉按钮点击）
    fn handle_move(&mut self, ui: &mut egui::Ui, frame_rect: egui::Rect, theme: &WindowTheme) {
        let drag_width = (self.header_right_x - frame_rect.left()).max(0.0);
        let header_rect =
            egui::Rect::from_min_size(frame_rect.min, egui::vec2(drag_width, theme.header_height));
        let resp = ui.interact(
            header_rect,
            self.id.with("header_drag"),
            egui::Sense::drag(),
        );
        if resp.dragged() {
            if let Some(pos) = &mut self.pos {
                *pos += resp.drag_delta();
            }
        }
    }

    /// 四边 + 四角 resize
    fn handle_resize(&mut self, ui: &mut egui::Ui, rect: egui::Rect, theme: &WindowTheme) {
        let size = self.size.unwrap_or(self.default_size);
        let min = self.min_size;
        // (grab_rect, left, top, right, bottom, cursor)
        let zones: [(egui::Rect, bool, bool, bool, bool, egui::CursorIcon); 8] = [
            // 右
            (
                egui::Rect::from_x_y_ranges(
                    rect.right() - GRAB..=rect.right() + GRAB,
                    rect.top() + GRAB..=rect.bottom() - GRAB,
                ),
                false,
                false,
                true,
                false,
                egui::CursorIcon::ResizeHorizontal,
            ),
            // 下
            (
                egui::Rect::from_x_y_ranges(
                    rect.left() + GRAB..=rect.right() - GRAB,
                    rect.bottom() - GRAB..=rect.bottom() + GRAB,
                ),
                false,
                false,
                false,
                true,
                egui::CursorIcon::ResizeVertical,
            ),
            // 左
            (
                egui::Rect::from_x_y_ranges(
                    rect.left() - GRAB..=rect.left() + GRAB,
                    rect.top() + GRAB..=rect.bottom() - GRAB,
                ),
                true,
                false,
                false,
                false,
                egui::CursorIcon::ResizeHorizontal,
            ),
            // 上
            (
                egui::Rect::from_x_y_ranges(
                    rect.left() + GRAB..=rect.right() - GRAB,
                    rect.top() - GRAB..=rect.top() + GRAB,
                ),
                false,
                true,
                false,
                false,
                egui::CursorIcon::ResizeVertical,
            ),
            // 右下
            (
                egui::Rect::from_x_y_ranges(
                    rect.right() - GRAB..=rect.right() + GRAB,
                    rect.bottom() - GRAB..=rect.bottom() + GRAB,
                ),
                false,
                false,
                true,
                true,
                egui::CursorIcon::ResizeNwSe,
            ),
            // 左上
            (
                egui::Rect::from_x_y_ranges(
                    rect.left() - GRAB..=rect.left() + GRAB,
                    rect.top() - GRAB..=rect.top() + GRAB,
                ),
                true,
                true,
                false,
                false,
                egui::CursorIcon::ResizeNwSe,
            ),
            // 右上
            (
                egui::Rect::from_x_y_ranges(
                    rect.right() - GRAB..=rect.right() + GRAB,
                    rect.top() - GRAB..=rect.top() + GRAB,
                ),
                false,
                true,
                true,
                false,
                egui::CursorIcon::ResizeNeSw,
            ),
            // 左下
            (
                egui::Rect::from_x_y_ranges(
                    rect.left() - GRAB..=rect.left() + GRAB,
                    rect.bottom() - GRAB..=rect.bottom() + GRAB,
                ),
                true,
                false,
                false,
                true,
                egui::CursorIcon::ResizeNeSw,
            ),
        ];
        let mut new_size = size;
        let mut pos_delta = egui::Vec2::ZERO;
        let mut any_active = false;
        for (i, &(zone, left, top, right, bottom, cursor)) in zones.iter().enumerate() {
            let resp = ui.interact(zone, self.id.with("rz").with(i), egui::Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(cursor);
            }
            if !resp.dragged() {
                continue;
            }
            any_active = true;
            let d = resp.drag_delta();
            if right {
                new_size.x += d.x;
            }
            if bottom {
                new_size.y += d.y;
            }
            if left {
                let w = (size.x - d.x).max(min.x);
                pos_delta.x += size.x - w;
                new_size.x = w;
            }
            if top {
                let h = (size.y - d.y).max(min.y);
                pos_delta.y += size.y - h;
                new_size.y = h;
            }
        }
        if any_active {
            new_size.x = new_size.x.max(min.x);
            new_size.y = new_size.y.max(min.y);
            self.size = Some(new_size);
            if let Some(pos) = &mut self.pos {
                *pos += pos_delta;
            }
            ui.ctx().request_repaint();
        }
        paint_resize_grip(ui.painter(), rect, theme.text_muted);
    }

    /// 渲染 header
    fn render_header(
        &mut self,
        ui: &mut egui::Ui,
        theme: &WindowTheme,
        header_right: impl FnOnce(&mut egui::Ui),
    ) {
        ui.horizontal(|ui| {
            ui.set_height(theme.header_height);
            ui.add_space(10.0);
            if let Some(icon) = self.icon {
                ui.label(
                    egui::RichText::new(icon.to_string())
                        .font(egui::FontId::new(14.0, theme.icon_font.clone()))
                        .color(theme.accent),
                );
                ui.add_space(6.0);
            }
            ui.label(
                egui::RichText::new(&self.title)
                    .font(egui::FontId::proportional(13.0))
                    .color(theme.text_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                self.render_pin_button(ui, theme);
                ui.add_space(8.0);
                header_right(ui);
                self.header_right_x = ui.min_rect().left();
            });
        });
    }

    fn render_pin_button(&mut self, ui: &mut egui::Ui, theme: &WindowTheme) {
        let is_pinned = self.pinned;
        let color = if is_pinned {
            theme.accent
        } else {
            theme.text_muted
        };
        let base_fill = if is_pinned {
            theme.bg_active
        } else {
            egui::Color32::TRANSPARENT
        };
        let resp = ui
            .scope(|ui| {
                let wv = &mut ui.style_mut().visuals.widgets;
                wv.inactive.weak_bg_fill = base_fill;
                wv.hovered.weak_bg_fill = theme.bg_hover;
                wv.active.weak_bg_fill = theme.bg_pressed;
                ui.add(
                    egui::Button::new(
                        egui::RichText::new(theme.pin_icon.to_string())
                            .font(egui::FontId::new(14.0, theme.icon_font.clone()))
                            .color(color),
                    )
                    .corner_radius(3)
                    .min_size(egui::vec2(26.0, 24.0)),
                )
            })
            .inner;
        if resp
            .on_hover_text(if is_pinned { "Unpin" } else { "Pin" })
            .clicked()
        {
            self.pinned = !self.pinned;
        }
    }
}

fn apply_style(ui: &mut egui::Ui, theme: &WindowTheme) {
    let style = ui.style_mut();
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme.text_primary);
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.hovered.bg_fill = theme.bg_hover;
    style.visuals.widgets.active.bg_fill = theme.bg_pressed;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
}

/// 绘制 1px 水平分隔线
pub fn separator(ui: &mut egui::Ui, color: egui::Color32) {
    let avail = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(avail.left(), avail.top()),
            egui::pos2(avail.right(), avail.top()),
        ],
        egui::Stroke::new(1.0, color),
    );
    ui.allocate_space(egui::vec2(avail.width(), 1.0));
}

/// 右下角 resize grip 指示线
fn paint_resize_grip(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let c = egui::Color32::from_white_alpha(60).lerp_to_gamma(color, 0.5);
    let stroke = egui::Stroke::new(1.0, c);
    let br = rect.right_bottom();
    for i in 1..=3 {
        let offset = i as f32 * 3.5;
        painter.line_segment(
            [
                egui::pos2(br.x - offset, br.y - 1.5),
                egui::pos2(br.x - 1.5, br.y - offset),
            ],
            stroke,
        );
    }
}
