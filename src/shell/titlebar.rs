//! 自定义标题栏：拖拽、双击最大化、Codicon 控制按钮
//!
//! @author sky

use super::{codicon, theme};
use eframe::egui;

/// 渲染标题栏，传入窗口标题文字
pub fn render(ui: &mut egui::Ui, title: &str) {
    let ctx = ui.ctx().clone();
    egui::Panel::top("title_bar")
        .frame(egui::Frame::NONE.fill(theme::BG_DARK))
        .show_inside(ui, |ui| {
            ui.set_height(theme::TITLE_BAR_HEIGHT);
            // 拖拽交互先绘制（底层），按钮后绘制（顶层）获得优先响应
            let title_bar_rect = ui.max_rect();
            let title_bar_response = ui.interact(
                title_bar_rect,
                ui.id().with("title_bar"),
                egui::Sense::click(),
            );
            #[cfg(not(target_os = "macos"))]
            if title_bar_response.double_clicked() {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if title_bar_response.is_pointer_button_down_on() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            #[cfg(target_os = "macos")]
            if title_bar_response.is_pointer_button_down_on() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            ui.horizontal_centered(|ui| {
                // macOS: 为原生交通灯预留空间
                // TODO: 用 eframe::native::macos::WindowChromeMetrics 精确测量
                #[cfg(target_os = "macos")]
                ui.add_space(72.0);
                #[cfg(not(target_os = "macos"))]
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(title)
                        .size(14.0)
                        .color(theme::TEXT_PRIMARY),
                );
                #[cfg(not(target_os = "macos"))]
                caption_buttons(ui, &ctx);
            });
        });
}

/// Windows / Linux 标题栏控制按钮组
#[cfg(not(target_os = "macos"))]
fn caption_buttons(ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if caption_button(ui, CaptionIcon::Close) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        let icon = if maximized {
            CaptionIcon::Restore
        } else {
            CaptionIcon::Maximize
        };
        if caption_button(ui, icon) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        if caption_button(ui, CaptionIcon::Minimize) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    });
}

#[derive(Clone, Copy)]
enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// 单个标题栏按钮（46x36，Codicon 图标，hover 分色）
fn caption_button(ui: &mut egui::Ui, icon: CaptionIcon) -> bool {
    let size = egui::vec2(46.0, theme::TITLE_BAR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let is_close = matches!(icon, CaptionIcon::Close);
    let painter = ui.painter();
    if hovered {
        let color = if is_close {
            theme::CLOSE_HOVER
        } else {
            theme::CAPTION_HOVER
        };
        painter.rect_filled(rect, 0.0, color);
    }
    let ic = if hovered && is_close {
        egui::Color32::WHITE
    } else {
        theme::TEXT_SECONDARY
    };
    let glyph = match icon {
        CaptionIcon::Minimize => codicon::CHROME_MINIMIZE,
        CaptionIcon::Maximize => codicon::CHROME_MAXIMIZE,
        CaptionIcon::Restore => codicon::CHROME_RESTORE,
        CaptionIcon::Close => codicon::CHROME_CLOSE,
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::new(16.0, codicon::family()),
        ic,
    );
    response.clicked()
}
