//! 自定义标题栏：拖拽 + 控制按钮
//!
//! 布局：[菜单栏(外部注入)] ... [标题居中] ... [最小化][最大化][关闭]
//! 拖拽在底层先注册，菜单和按钮在顶层后注册覆盖拖拽。
//!
//! @author sky

use super::{codicon, theme};
use eframe::egui;

/// caption button 宽度
const CAPTION_BTN_W: f32 = 46.0;

/// 渲染标题栏，菜单内容由调用方通过闭包注入
pub fn render(ui: &mut egui::Ui, title: &str, menu_bar: impl FnOnce(&mut egui::Ui)) {
    let ctx = ui.ctx().clone();
    egui::Panel::top("title_bar")
        .frame(egui::Frame::NONE.fill(theme::BG_DARK))
        .show_separator_line(false)
        .show_inside(ui, |ui| {
            ui.set_height(theme::TITLE_BAR_HEIGHT);
            let bar = ui.max_rect();
            // 底层：拖拽区域（先注册 = 优先级最低，被后续控件覆盖）
            let drag = ui.interact(bar, ui.id().with("drag"), egui::Sense::click_and_drag());
            #[cfg(not(target_os = "macos"))]
            if drag.double_clicked() {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            #[cfg(target_os = "macos")]
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            // 菜单栏（左侧，排除右侧 caption buttons 区域）
            #[cfg(not(target_os = "macos"))]
            let menu_w = bar.width() - CAPTION_BTN_W * 3.0;
            #[cfg(target_os = "macos")]
            let menu_w = bar.width();
            let menu_rect =
                egui::Rect::from_min_size(bar.left_top(), egui::vec2(menu_w, bar.height()));
            ui.new_child(egui::UiBuilder::new().max_rect(menu_rect))
                .horizontal_centered(|ui| {
                    // 菜单按钮样式：hover 时背景变亮 + 文字变亮
                    let visuals = &mut ui.style_mut().visuals;
                    visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                    visuals.widgets.inactive.fg_stroke =
                        egui::Stroke::new(1.0, theme::TEXT_SECONDARY);
                    visuals.widgets.hovered.weak_bg_fill = theme::BG_HOVER;
                    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_PRIMARY);
                    visuals.widgets.active.weak_bg_fill = theme::BG_HOVER;
                    visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_PRIMARY);
                    ui.spacing_mut().item_spacing.x = 0.0;
                    #[cfg(target_os = "macos")]
                    ui.add_space(72.0);
                    #[cfg(not(target_os = "macos"))]
                    ui.add_space(8.0);
                    menu_bar(ui);
                });
            // caption buttons（右侧，仅 Windows/Linux）
            #[cfg(not(target_os = "macos"))]
            {
                let w = CAPTION_BTN_W * 3.0;
                let rect = egui::Rect::from_min_size(
                    egui::pos2(bar.right() - w, bar.top()),
                    egui::vec2(w, bar.height()),
                );
                caption_buttons(
                    &mut ui.new_child(egui::UiBuilder::new().max_rect(rect)),
                    &ctx,
                );
            }
            // 标题居中（纯绘制）
            ui.painter().text(
                bar.center(),
                egui::Align2::CENTER_CENTER,
                title,
                egui::FontId::proportional(14.0),
                theme::VERDIGRIS,
            );
        });
}

#[cfg(not(target_os = "macos"))]
fn caption_buttons(ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if caption_button(ui, Caption::Minimize) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let icon = if maximized {
            Caption::Restore
        } else {
            Caption::Maximize
        };
        if caption_button(ui, icon) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        if caption_button(ui, Caption::Close) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

#[cfg(not(target_os = "macos"))]
#[derive(Clone, Copy)]
enum Caption {
    Minimize,
    Maximize,
    Restore,
    Close,
}

#[cfg(not(target_os = "macos"))]
impl Caption {
    fn glyph(self) -> &'static str {
        match self {
            Self::Minimize => codicon::CHROME_MINIMIZE,
            Self::Maximize => codicon::CHROME_MAXIMIZE,
            Self::Restore => codicon::CHROME_RESTORE,
            Self::Close => codicon::CHROME_CLOSE,
        }
    }
    fn is_close(self) -> bool {
        matches!(self, Self::Close)
    }
}

#[cfg(not(target_os = "macos"))]
fn caption_button(ui: &mut egui::Ui, icon: Caption) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(CAPTION_BTN_W, theme::TITLE_BAR_HEIGHT),
        egui::Sense::click(),
    );
    if response.hovered() {
        let bg = if icon.is_close() {
            theme::CLOSE_HOVER
        } else {
            theme::CAPTION_HOVER
        };
        ui.painter().rect_filled(rect, 0.0, bg);
    }
    let fg = if response.hovered() && icon.is_close() {
        egui::Color32::WHITE
    } else {
        theme::TEXT_SECONDARY
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph(),
        egui::FontId::new(13.0, codicon::family()),
        fg,
    );
    response.clicked()
}
