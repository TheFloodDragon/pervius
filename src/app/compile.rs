//! Java 源码编译任务调度与轮询
//!
//! @author sky

use super::App;
use crate::task::{Poll, Pollable, Task};
use pervius_java_bridge::compiler::{self, CompileOutcome, DiagSeverity};
use rust_i18n::t;

impl App {
    /// 启动指定 class tab 的源码编译
    pub(crate) fn compile_source_tab(&mut self, entry_path: &str) {
        let mut source = None;
        for (_, tab) in self.layout.editor.dock_state.iter_all_tabs_mut() {
            if tab.entry_path.as_deref() == Some(entry_path) {
                if !tab.is_class || !tab.is_source_unlocked() {
                    return;
                }
                if tab.is_modified {
                    self.toasts
                        .warning(t!("editor.source_vs_struct_conflict"));
                    return;
                }
                tab.compile_diagnostics.clear();
                source = Some(tab.decompiled.clone());
                break;
            }
        }
        let Some(source) = source else {
            return;
        };
        if !compiler::is_jdk_available() {
            self.toasts.error(t!("editor.jdk_required"));
            return;
        }
        let Some(binary_name) = entry_path.strip_suffix(".class").map(str::to_string) else {
            return;
        };
        let jar_path = self.workspace.jar().map(|j| j.path.clone());
        let target = Some(self.settings.compile.target_version);
        let debug = self.settings.compile.emit_debug_info;
        let task = Task::spawn(move || {
            compiler::compile_source(
                &source,
                &binary_name,
                jar_path.as_deref(),
                target,
                debug,
            )
        });
        self.pending_compiles.push((entry_path.to_string(), task));
        log::info!("Compiling source: {entry_path}");
    }

    /// 轮询 class 源码编译结果，成功后写回 JAR 并刷新反编译源码
    pub(crate) fn poll_class_compiles(&mut self) {
        if self.pending_compiles.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.pending_compiles.len() {
            let result = match self.pending_compiles[i].1.poll() {
                Poll::Ready(r) => r,
                Poll::Pending => {
                    i += 1;
                    continue;
                }
                Poll::Lost => {
                    self.pending_compiles.swap_remove(i);
                    continue;
                }
            };
            let entry_path = self.pending_compiles.swap_remove(i).0;
            match result {
                Ok(CompileOutcome::Success(classes)) => {
                    let mut main_bytes = None;
                    let main_binary = entry_path.trim_end_matches(".class");
                    let count = classes.len();
                    for class in classes {
                        let class_path = format!("{}.class", class.binary_name);
                        if class.binary_name == main_binary {
                            main_bytes = Some(class.bytes.clone());
                        }
                        if let Some(jar) = self.workspace.jar_mut() {
                            jar.put(&class_path, class.bytes.clone());
                        }
                        for (_, tab) in self.layout.editor.dock_state.iter_all_tabs_mut() {
                            if tab.entry_path.as_deref() == Some(&class_path) {
                                tab.commit_save(class.bytes.clone());
                                if class_path == entry_path {
                                    tab.source_modified = false;
                                    tab.original_source = tab.decompiled.clone();
                                    tab.compile_diagnostics.clear();
                                }
                            }
                        }
                    }
                    if let Some(bytes) = main_bytes {
                        self.decompile_class(&entry_path, bytes, false);
                    }
                    self.toasts
                        .success(t!("editor.recompile_success", n = count));
                    log::info!("Compiled source: {entry_path} ({count} classes)");
                }
                Ok(CompileOutcome::Errors(diagnostics)) => {
                    let first = diagnostics
                        .iter()
                        .find(|d| d.severity == DiagSeverity::Error)
                        .or_else(|| diagnostics.first());
                    for (_, tab) in self.layout.editor.dock_state.iter_all_tabs_mut() {
                        if tab.entry_path.as_deref() == Some(&entry_path) {
                            tab.compile_diagnostics = diagnostics.clone();
                            if let Some(diag) = first {
                                if diag.line > 0 {
                                    tab.pending_scroll_to_line = Some(diag.line.saturating_sub(1) as usize);
                                }
                            }
                            break;
                        }
                    }
                    let message = first
                        .map(|d| {
                            if d.line > 0 {
                                format!("{}:{} {}", d.line, d.column, d.message)
                            } else {
                                d.message.clone()
                            }
                        })
                        .unwrap_or_else(|| "javac reported errors".to_string());
                    self.toasts
                        .error(t!("editor.recompile_failed", error = message));
                    log::warn!("Compile failed: {entry_path}");
                }
                Ok(CompileOutcome::JdkMissing) => {
                    self.toasts.error(t!("editor.jdk_required"));
                }
                Err(e) => {
                    log::error!("Compile task failed: {entry_path}: {e}");
                    self.toasts
                        .error(t!("editor.recompile_failed", error = e.to_string()));
                }
            }
        }
    }
}
