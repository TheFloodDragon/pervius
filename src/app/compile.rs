//! Java 源码编译任务调度与轮询
//!
//! @author sky

use super::App;
use crate::task::{Poll, Pollable, Task};
use egui_editor::highlight::Language;
use pervius_java_bridge::compiler::{self, CompileOutcome, DiagSeverity, KotlinSource};
use pervius_java_bridge::error::BridgeError;
use rust_i18n::t;

pub(crate) struct PendingCompile {
    /// 触发编译的 tab/class 路径。
    pub entry_path: String,
    /// 编译开始时的源码快照，用于避免异步完成后覆盖用户的新编辑。
    pub source_snapshot: String,
    pub task: Task<Result<CompileOutcome, BridgeError>>,
}

fn class_info_release(version: Option<&str>) -> Option<u8> {
    let version = version?;
    let java = version.strip_prefix("Java ")?;
    let release = java.split_whitespace().next()?;
    let release = release.strip_prefix("1.").unwrap_or(release);
    release.parse::<u8>().ok()
}

fn kotlin_source_path(entry_path: &str) -> String {
    let base = entry_path.strip_suffix(".class").unwrap_or(entry_path);
    let outer = base.find('$').map_or(base, |idx| &base[..idx]);
    format!("{outer}.kt")
}

impl App {
    /// 启动指定 class tab 的源码编译
    pub(crate) fn compile_source_tab(&mut self, entry_path: &str) {
        let mut source = None;
        let mut language = Language::Java;
        let mut target = None;
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
                language = tab.language;
                target = class_info_release(tab.class_info.as_deref());
                break;
            }
        }
        let Some(source) = source else {
            return;
        };
        let Some(binary_name) = entry_path.strip_suffix(".class").map(str::to_string) else {
            return;
        };
        if language != Language::Kotlin && !compiler::is_jdk_available() {
            self.toasts.error(t!("editor.jdk_required"));
            return;
        }
        let Some(target) = target else {
            self.toasts.error(t!(
                "editor.recompile_failed",
                error = "Cannot determine original class version"
            ));
            return;
        };
        let jar_path = self.workspace.jar().map(|j| j.path.clone());
        let target = Some(target);
        let kotlin_skip_metadata_version_check = self
            .settings
            .java
            .kotlin_skip_metadata_version_check;
        let source_entry = entry_path.to_string();
        let source_snapshot = source.clone();
        let task = Task::spawn(move || {
            if language == Language::Kotlin {
                let kt_path = kotlin_source_path(&source_entry);
                let sources = [KotlinSource {
                    path: kt_path,
                    source,
                }];
                compiler::compile_kotlin_sources_with_options(
                    &sources,
                    jar_path.as_deref(),
                    target,
                    None,
                    kotlin_skip_metadata_version_check,
                )
            } else {
                compiler::compile_source(
                    &source,
                    &binary_name,
                    jar_path.as_deref(),
                    target,
                    true,
                )
            }
        });
        self.pending_compiles.push(PendingCompile {
            entry_path: entry_path.to_string(),
            source_snapshot,
            task,
        });
        log::info!("Compiling source: {entry_path}");
    }

    fn compile_source_snapshot_is_current(&self, entry_path: &str, source_snapshot: &str) -> bool {
        self.layout
            .editor
            .dock_state
            .iter_all_tabs()
            .find(|(_, tab)| tab.entry_path.as_deref() == Some(entry_path))
            .map(|(_, tab)| tab.decompiled == source_snapshot)
            .unwrap_or(true)
    }

    /// 轮询 class 源码编译结果，成功后写回 JAR 并刷新反编译源码
    pub(crate) fn poll_class_compiles(&mut self) {
        if self.pending_compiles.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.pending_compiles.len() {
            let result = match self.pending_compiles[i].task.poll() {
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
            let pending = self.pending_compiles.swap_remove(i);
            let entry_path = pending.entry_path;
            let source_snapshot = pending.source_snapshot;
            match result {
                Ok(CompileOutcome::Success(classes)) => {
                    if !self.compile_source_snapshot_is_current(&entry_path, &source_snapshot) {
                        log::warn!("Discarding stale compile result: {entry_path}");
                        continue;
                    }
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
                                tab.source_modified = false;
                                if class_path == entry_path {
                                    tab.decompiled = source_snapshot.clone();
                                    tab.original_source = source_snapshot.clone();
                                    tab.refresh_decompiled_data();
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
