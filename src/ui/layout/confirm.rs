//! 未保存变更确认对话框（模态遮罩 + 居中面板）
//!
//! @author sky

use super::Layout;
use crate::shell::theme;
use eframe::egui;
use rust_i18n::t;
use std::path::PathBuf;

/// 需要用户确认的破坏性动作
pub enum ConfirmAction {
    Close,
    OpenDialog,
    Open(PathBuf),
}

impl Layout {
    /// 是否有未保存的变更（tab 级别编辑 或 JAR 内存级别修改）
    pub fn has_unsaved_changes(&self) -> bool {
        self.jar.as_ref().is_some_and(|j| j.has_modified_entries())
            || self
                .editor
                .dock_state
                .iter_all_tabs()
                .any(|(_, tab)| tab.is_modified)
    }

    /// 带确认的关闭请求
    pub fn request_close(&mut self, ctx: &egui::Context) {
        if self.has_unsaved_changes() {
            self.pending_confirm = Some(ConfirmAction::Close);
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// 带确认的打开 JAR 对话框
    pub fn request_open_jar_dialog(&mut self) {
        if self.has_unsaved_changes() {
            self.pending_confirm = Some(ConfirmAction::OpenDialog);
        } else {
            self.open_jar_dialog();
        }
    }

    /// 带确认的打开指定 JAR
    pub fn request_open_jar(&mut self, path: &std::path::Path) {
        if self.has_unsaved_changes() {
            self.pending_confirm = Some(ConfirmAction::Open(path.to_path_buf()));
        } else {
            self.open_jar(path);
        }
    }

    /// 执行已确认的动作
    fn execute_confirmed(&mut self, action: ConfirmAction, ctx: &egui::Context) {
        match action {
            ConfirmAction::Close => {
                // 清除所有未保存标记，避免下一帧再次拦截关闭
                for (_, tab) in self.editor.dock_state.iter_all_tabs_mut() {
                    tab.is_modified = false;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ConfirmAction::OpenDialog => {
                self.open_jar_dialog();
            }
            ConfirmAction::Open(path) => {
                self.open_jar(&path);
            }
        }
    }

    /// 渲染确认对话框（模态遮罩 + 居中面板）
    pub(super) fn render_confirm(&mut self, ctx: &egui::Context) {
        if self.pending_confirm.is_none() {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.pending_confirm = None;
            return;
        }
        let screen = ctx.screen_rect();
        // 半透明遮罩
        let backdrop_layer =
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("confirm_backdrop"));
        ctx.layer_painter(backdrop_layer).rect_filled(
            screen,
            0.0,
            egui::Color32::from_black_alpha(120),
        );
        // 对话框
        let mut discard = false;
        let dialog_resp = egui::Area::new(egui::Id::new("confirm_dialog"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme::BG_GUTTER)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .corner_radius(8.0)
                    .show(ui, |ui| {
                        ui.set_width(340.0);
                        // 内容区
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(t!("confirm.unsaved_title"))
                                        .size(13.0)
                                        .color(theme::TEXT_PRIMARY)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(t!("confirm.unsaved_message"))
                                        .size(12.0)
                                        .color(theme::TEXT_SECONDARY),
                                );
                            });
                            ui.add_space(20.0);
                        });
                        ui.add_space(16.0);
                        // 分隔线
                        let sep_rect = egui::Rect::from_min_size(
                            egui::pos2(ui.max_rect().left(), ui.cursor().top()),
                            egui::vec2(ui.available_width(), 1.0),
                        );
                        ui.painter().rect_filled(sep_rect, 0.0, theme::BORDER);
                        ui.add_space(1.0);
                        // 按钮区
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(20.0);
                                    if ui
                                        .add(
                                            crate::ui::widget::FlatButton::new(&t!(
                                                "confirm.cancel"
                                            ))
                                            .min_size(egui::vec2(72.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        self.pending_confirm = None;
                                    }
                                    ui.add_space(6.0);
                                    if ui
                                        .add(
                                            crate::ui::widget::FlatButton::new(&t!(
                                                "confirm.discard"
                                            ))
                                            .inactive_color(theme::ACCENT_RED)
                                            .min_size(egui::vec2(88.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        discard = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(12.0);
                    });
            });
        if discard {
            let action = self.pending_confirm.take().unwrap();
            self.execute_confirmed(action, ctx);
            return;
        }
        // 点击对话框外部关闭
        if ctx.input(|i| i.pointer.any_pressed()) {
            let pointer = ctx.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer {
                if !dialog_resp.response.rect.contains(pos) {
                    self.pending_confirm = None;
                }
            }
        }
    }
}
