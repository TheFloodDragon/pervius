//! 未保存变更确认：业务逻辑 + 动作分发
//!
//! UI 渲染委托给 `egui_shell::components::ConfirmDialog`。
//!
//! @author sky

use super::editor::TabAction;
use crate::app::App;
use crate::appearance::theme;
use eframe::egui;
use egui_shell::components::{ConfirmDialog, ConfirmResult};
use rust_i18n::t;
use std::path::PathBuf;

/// 需要用户确认的破坏性动作
pub enum ConfirmAction {
    /// 关闭整个应用窗口
    Close,
    /// 打开文件选择器加载新 JAR
    OpenDialog,
    /// 打开指定 JAR 文件
    Open(PathBuf),
    /// 关闭 tab（单个/批量）
    TabClose(TabAction),
    /// 大 JAR 全量反编译确认
    DecompileAll,
}

impl App {
    /// 是否有未保存的变更（tab 级别编辑 或 JAR 内存级别修改）
    pub fn has_unsaved_changes(&self) -> bool {
        self.workspace
            .jar()
            .is_some_and(|j| j.has_modified_entries())
            || self
                .layout
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
            ConfirmAction::Close | ConfirmAction::OpenDialog | ConfirmAction::Open(_) => {
                // 全局动作：清除所有未保存标记，避免 has_unsaved_changes() 再次拦截
                for (_, tab) in self.layout.editor.dock_state.iter_all_tabs_mut() {
                    tab.is_modified = false;
                }
                if let Some(jar) = self.workspace.jar_mut() {
                    jar.clear_modified();
                }
                match action {
                    ConfirmAction::Close => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ConfirmAction::OpenDialog => {
                        self.open_jar_dialog();
                    }
                    ConfirmAction::Open(path) => {
                        self.open_jar(&path);
                    }
                    _ => unreachable!(),
                }
            }
            ConfirmAction::TabClose(tab_action) => {
                self.layout.editor.force_tab_action(tab_action);
            }
            ConfirmAction::DecompileAll => {
                self.start_confirmed_decompile();
            }
        }
    }

    /// 渲染确认弹窗（根据 ConfirmAction 类型选择标题/按钮）
    pub(crate) fn render_confirm(&mut self, ctx: &egui::Context) {
        let Some(action) = &self.pending_confirm else {
            return;
        };
        let is_decompile = matches!(action, ConfirmAction::DecompileAll);
        let (title, message, confirm_label, cancel_label, confirm_color);
        if is_decompile {
            let (size_mb, count) = self
                .workspace
                .jar()
                .map(|j| (j.file_size as f64 / 1_000_000.0, j.class_count()))
                .unwrap_or((0.0, 0));
            let size_str = if size_mb >= 10.0 {
                format!("{size_mb:.0}")
            } else {
                format!("{size_mb:.1}")
            };
            title = t!("confirm.decompile_title");
            message = t!("confirm.decompile_message", size = size_str, count = count);
            confirm_label = t!("confirm.decompile_yes");
            cancel_label = t!("confirm.decompile_skip");
            confirm_color = None;
        } else {
            title = t!("confirm.unsaved_title");
            message = t!("confirm.unsaved_message");
            confirm_label = t!("confirm.discard");
            cancel_label = t!("confirm.cancel");
            confirm_color = Some(theme::ACCENT_RED);
        };
        let mut dialog = ConfirmDialog::new(&title, &message)
            .confirm_label(&confirm_label)
            .cancel_label(&cancel_label);
        if let Some(color) = confirm_color {
            dialog = dialog.confirm_color(color);
        }
        let result = dialog.show(ctx, &theme::confirm_theme());
        match result {
            ConfirmResult::Confirmed => {
                let action = self.pending_confirm.take().unwrap();
                self.execute_confirmed(action, ctx);
            }
            ConfirmResult::Dismissed => {
                self.pending_confirm = None;
            }
            ConfirmResult::None => {}
        }
    }
}
