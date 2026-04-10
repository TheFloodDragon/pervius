//! 反编译任务调度与轮询
//!
//! @author sky

use super::Layout;
use egui_editor::highlight::Language;
use pervius_java_bridge::decompiler;
use rust_i18n::t;
use std::collections::HashSet;
use std::sync::mpsc;

impl Layout {
    /// 检查后台反编译是否完成
    pub(super) fn check_decompiling(&mut self) {
        let Some(task) = &self.decompiling else {
            return;
        };
        // 快照已反编译类集合
        if let Ok(set) = task.progress.decompiled.lock() {
            if !set.is_empty() {
                self.decompiled_classes = Some(set.clone());
            }
        }
        let jar_name = task.jar_name.clone();
        let result = poll_recv!(task.receiver,
            miss => return,
            disconnect => { log::error!("Decompiler thread disconnected"); self.decompiling = None; return; }
        );
        self.decompiling = None;
        match result {
            Ok(()) => {
                log::info!("Decompilation complete: {jar_name}");
                let hash = self.jar.as_ref().map(|j| j.hash.as_str());
                self.editor.refresh_class_tabs(hash);
                self.toasts
                    .info(t!("layout.decompile_complete", name = jar_name));
                self.decompiled_classes = None;
            }
            Err(e) => {
                log::error!("Decompilation failed: {e}");
                self.toasts.error(t!("layout.decompile_failed", error = e));
            }
        }
    }

    /// JAR 加载完成后启动反编译（有缓存则跳过）
    pub(super) fn start_decompile(&mut self, jar: &pervius_java_bridge::jar::JarArchive) {
        if decompiler::is_cached(&jar.hash) {
            log::info!("Decompiled cache hit for {}", jar.name);
            self.decompiled_classes = None;
            return;
        }
        self.decompiled_classes = Some(HashSet::new());
        self.force_decompile(jar);
    }

    /// 强制重反编译（清除缓存后重新启动）
    pub fn re_decompile(&mut self) {
        let Some(jar) = &self.jar else { return };
        if self.decompiling.is_some() {
            self.toasts.warning(t!("layout.decompile_in_progress"));
            return;
        }
        self.decompiled_classes = Some(HashSet::new());
        let path = jar.path.clone();
        let name = jar.name.clone();
        let hash = jar.hash.clone();
        let class_count = jar.class_count();
        let (tx, rx) = mpsc::channel();
        let thread_name = name.clone();
        std::thread::spawn(move || {
            decompiler::clear_cache(&hash);
            let result = decompiler::start(&path, &thread_name, &hash, class_count);
            let _ = tx.send(result);
        });
        log::info!("Re-decompiling: {name} ({class_count} classes)");
        self.pending_re_decompile = Some((name, rx));
    }

    /// 启动反编译（无缓存检查）
    fn force_decompile(&mut self, jar: &pervius_java_bridge::jar::JarArchive) {
        match decompiler::start(&jar.path, &jar.name, &jar.hash, jar.class_count()) {
            Ok(task) => {
                log::info!(
                    "Starting decompilation: {} ({} classes)",
                    jar.name,
                    jar.class_count()
                );
                self.decompiling = Some(task);
            }
            Err(e) => {
                log::warn!("Cannot start decompiler: {e}");
                self.toasts
                    .warning(t!("layout.decompiler_unavailable", error = e));
            }
        }
    }

    /// 后台反编译单个 .class 文件
    ///
    /// `write_cache` 为 true 时输出写入缓存目录（首次预览），为 false 时仅返回内存结果。
    /// 有 JAR 上下文时自动传入 `-e` 依赖解析；独立文件无上下文也能反编译。
    pub(super) fn start_single_decompile(
        &mut self,
        entry_path: &str,
        bytes: Vec<u8>,
        write_cache: bool,
    ) {
        let jar_path = self.jar.as_ref().map(|j| j.path.clone());
        let hash = if write_cache {
            self.jar.as_ref().map(|j| j.hash.clone())
        } else {
            None
        };
        let class_path = entry_path.to_string();
        let (tx, rx) = mpsc::channel();
        let cp = class_path.clone();
        std::thread::spawn(move || {
            let result = decompiler::decompile_single_class(
                &bytes,
                &cp,
                jar_path.as_deref(),
                hash.as_deref(),
            );
            let _ = tx.send(result);
        });
        self.pending_decompiles.push((class_path, rx));
    }

    /// 轮询后台重反编译启动结果
    pub(super) fn check_re_decompile(&mut self) {
        let Some((_, rx)) = &self.pending_re_decompile else {
            return;
        };
        let result = poll_recv!(rx,
            miss => return,
            disconnect => { self.pending_re_decompile = None; return; }
        );
        self.pending_re_decompile = None;
        match result {
            Ok(task) => {
                self.decompiling = Some(task);
            }
            Err(e) => {
                log::error!("Re-decompile failed to start: {e}");
                self.toasts
                    .warning(t!("layout.decompiler_unavailable", error = e));
            }
        }
    }

    /// 轮询单文件反编译结果，完成后回填到对应 tab
    pub(super) fn check_single_decompile(&mut self) {
        if self.pending_decompiles.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.pending_decompiles.len() {
            let result = poll_recv!(self.pending_decompiles[i].1,
                miss => { i += 1; continue; },
                disconnect => { self.pending_decompiles.swap_remove(i); continue; }
            );
            let entry_path = self.pending_decompiles.swap_remove(i).0;
            match result {
                Ok(cached) => {
                    let lang = if cached.is_kotlin {
                        Language::Kotlin
                    } else {
                        Language::Java
                    };
                    for (_, tab) in self.editor.dock_state.iter_all_tabs_mut() {
                        if tab.entry_path.as_deref() == Some(&entry_path) {
                            tab.set_decompiled(
                                cached.source.clone(),
                                lang,
                                cached.line_mapping.clone(),
                            );
                            break;
                        }
                    }
                    // 已修改条目的反编译结果缓存到 JAR 内存（磁盘缓存无效）
                    if let Some(jar) = &mut self.jar {
                        if jar.is_modified(&entry_path) {
                            jar.put_decompiled(&entry_path, cached);
                        }
                    }
                    if let Some(set) = &mut self.decompiled_classes {
                        let base = entry_path.strip_suffix(".class").unwrap_or(&entry_path);
                        let base = match base.find('$') {
                            Some(pos) => &base[..pos],
                            None => base,
                        };
                        set.insert(base.to_string());
                        for (idx, _) in base.match_indices('/') {
                            set.insert(base[..idx + 1].to_string());
                        }
                    }
                    log::info!("Single-decompiled: {entry_path}");
                }
                Err(e) => {
                    log::warn!("Single decompile failed: {entry_path}: {e}");
                    self.toasts.error(t!("layout.decompile_failed", error = e));
                }
            }
        }
    }
}
