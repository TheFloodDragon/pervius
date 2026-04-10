//! JAR 文件操作：打开、异步加载、拖拽处理、tab 创建、编码检测、反编译调度
//!
//! @author sky

use super::Layout;
use crate::appearance::theme;
use crate::ui::editor::view_toggle::ActiveView;
use crate::ui::editor::{EditorArea, EditorTab};
use crate::ui::explorer::tree;
use eframe::egui;
use egui_editor::highlight::Language;
use egui_shell::components::SettingsFile;
use pervius_java_bridge::class_structure::SavedMember;
use pervius_java_bridge::decompiler::{self, CachedSource};
use pervius_java_bridge::error::BridgeError;
use pervius_java_bridge::jar::{JarArchive, LoadProgress};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};

/// JAR 后台加载状态
pub(super) struct LoadingState {
    pub name: String,
    pub progress: Arc<LoadProgress>,
    pub receiver: mpsc::Receiver<Result<JarArchive, BridgeError>>,
}

impl Layout {
    /// 处理 explorer 中右键「Reveal in Explorer」
    pub(super) fn handle_pending_reveal(&mut self) {
        let entry_path = match self.file_panel.pending_reveal.take() {
            Some(p) => p,
            None => return,
        };
        let hash = match &self.jar {
            Some(j) => &j.hash,
            None => return,
        };
        log::info!("Reveal: entry_path={entry_path}");
        // class 文件：定位到缓存的反编译源码
        if entry_path.ends_with(".class") || entry_path.contains('$') {
            if let Some(file) = decompiler::cached_source_path(hash, &entry_path) {
                log::info!("Reveal: found {}", file.display());
                reveal_in_explorer(&file);
                return;
            }
            log::warn!("Reveal: cached source not found for {entry_path}");
        }
        // 缓存未命中：直接打开缓存目录（显示内容）
        if let Ok(dir) = decompiler::cache_dir(hash) {
            if dir.exists() {
                log::info!("Reveal: fallback to dir {}", dir.display());
                open_directory(&dir);
            }
        }
    }

    /// 处理拖拽到窗口的文件
    pub(super) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        // 多 pass 去重
        let frame = ctx.cumulative_frame_nr();
        let cache_id = egui::Id::new("drop_frame");
        let last: Option<u64> = ctx.data(|d| d.get_temp(cache_id));
        if last == Some(frame) {
            return;
        }
        ctx.data_mut(|d| d.insert_temp(cache_id, frame));
        if let Some(file) = dropped.into_iter().next() {
            if let Some(path) = &file.path {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                match ext.to_ascii_lowercase().as_str() {
                    "jar" | "zip" | "war" | "ear" => self.request_open_jar(path),
                    _ => {}
                }
            }
        }
    }

    /// 处理 explorer 中点击的文件
    pub(super) fn handle_pending_open(&mut self) {
        let entry_path = match self.file_panel.pending_open.take() {
            Some(p) => p,
            None => return,
        };
        // 已打开的 tab 直接聚焦
        if self.editor.focus_tab(&entry_path) {
            return;
        }
        let bytes = match self.jar.as_ref().and_then(|j| j.get(&entry_path)) {
            Some(b) => b.to_vec(),
            None => return,
        };
        let jar = self.jar.as_ref().unwrap();
        let hash = jar.hash.as_str();
        let is_modified = jar.is_modified(&entry_path);
        // 已修改条目优先从 JAR 内存缓存读取反编译结果
        let mem_cached = if is_modified {
            jar.get_decompiled(&entry_path).cloned()
        } else {
            None
        };
        let has_cache = !is_modified && decompiler::cached_source_path(hash, &entry_path).is_some();
        let tab = Self::create_tab(&entry_path, &bytes, Some(hash), mem_cached.as_ref());
        self.editor.open_tab(tab);
        if entry_path.ends_with(".class") {
            if is_modified && mem_cached.is_none() {
                // 已修改但无内存缓存（首次保存后 tab 未关闭过），重新反编译
                self.start_single_decompile(&entry_path, false);
            } else if !is_modified && !has_cache {
                self.start_single_decompile(&entry_path, true);
            }
        }
    }

    /// 检查后台 JAR 加载是否完成
    pub(super) fn check_loading(&mut self) {
        let loading = match &self.loading {
            Some(l) => l,
            None => return,
        };
        match loading.receiver.try_recv() {
            Ok(Ok(jar)) => {
                let paths = jar.paths();
                self.file_panel.roots = tree::build_tree(&jar.name, &paths);
                self.file_panel.selected = None;
                self.file_panel.filter.clear();
                self.start_decompile(&jar);
                // 记入最近打开列表
                self.settings.add_recent(&jar.path, &jar.name);
                if let Err(e) = self.settings.save() {
                    log::warn!("保存最近打开记录失败: {e}");
                }
                self.jar = Some(jar);
                self.explorer_visible = true;
                self.loading = None;
            }
            Ok(Err(e)) => {
                log::error!("Failed to open JAR: {e}");
                self.loading = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.loading = None;
            }
        }
    }

    /// 检查后台反编译是否完成
    pub(super) fn check_decompiling(&mut self) {
        let task = match &self.decompiling {
            Some(t) => t,
            None => return,
        };
        // 快照已反编译类集合
        if let Ok(set) = task.progress.decompiled.lock() {
            if !set.is_empty() {
                self.decompiled_classes = Some(set.clone());
            }
        }
        match task.receiver.try_recv() {
            Ok(Ok(())) => {
                log::info!("Decompilation complete: {}", task.jar_name);
                let hash = self.jar.as_ref().map(|j| j.hash.as_str());
                self.editor.refresh_class_tabs(hash);
                self.toasts
                    .info(t!("layout.decompile_complete", name = task.jar_name));
                self.decompiling = None;
                self.decompiled_classes = None;
            }
            Ok(Err(e)) => {
                log::error!("Decompilation failed: {e}");
                self.toasts.error(t!("layout.decompile_failed", error = e));
                self.decompiling = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                log::error!("Decompiler thread disconnected");
                self.decompiling = None;
            }
        }
    }

    /// JAR 加载完成后启动反编译（有缓存则跳过）
    fn start_decompile(&mut self, jar: &JarArchive) {
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
        let jar = match &self.jar {
            Some(j) => j,
            None => return,
        };
        if self.decompiling.is_some() {
            self.toasts.warning(t!("layout.decompile_in_progress"));
            return;
        }
        self.decompiled_classes = Some(HashSet::new());
        let path = jar.path.clone();
        let name = jar.name.clone();
        let hash = jar.hash.clone();
        let class_count = jar.class_count();
        // 清缓存 + 启动反编译都在后台线程，避免 remove_dir_all 卡主线程
        let (tx, rx) = mpsc::channel();
        let thread_name = name.clone();
        std::thread::spawn(move || {
            decompiler::clear_cache(&hash);
            let result = decompiler::start(&path, &thread_name, &hash, class_count);
            let _ = tx.send(result);
        });
        match rx.try_recv() {
            // 不会立即就绪，存 receiver 等后续帧轮询
            _ => {
                log::info!("Re-decompiling: {name} ({class_count} classes)");
                self.pending_re_decompile = Some((name, rx));
            }
        }
    }

    /// 启动反编译（无缓存检查）
    fn force_decompile(&mut self, jar: &JarArchive) {
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

    /// 保存当前聚焦 tab 的修改到 JAR 内存，并触发单文件反编译
    pub fn save_active_tab(&mut self) {
        let tab = match self.editor.focused_tab_mut() {
            Some(t) => t,
            None => return,
        };
        if !tab.is_modified {
            return;
        }
        let entry_path = match tab.entry_path.clone() {
            Some(p) => p,
            None => return,
        };
        // 纯文本文件：直接将编辑后的文本写回 JAR
        if tab.is_text {
            let new_bytes = tab.decompiled.as_bytes().to_vec();
            tab.raw_bytes = new_bytes.clone();
            tab.is_modified = false;
            if let Some(jar) = &mut self.jar {
                jar.put(&entry_path, new_bytes);
            }
            log::info!("Saved text: {entry_path}");
            return;
        }
        let cs = match &tab.class_structure {
            Some(cs) => cs,
            None => return,
        };
        match pervius_java_bridge::save::apply_structure(
            &tab.raw_bytes,
            cs,
            self.jar.as_ref().map(|j| j.path.as_path()),
        ) {
            Ok(new_bytes) => {
                tab.raw_bytes = new_bytes.clone();
                tab.is_modified = false;
                // 就地翻转 modified → saved，不重建 class structure
                let mut saved_set: HashSet<SavedMember> = HashSet::new();
                if let Some(cs) = &mut tab.class_structure {
                    if cs.info.modified || cs.info.saved {
                        cs.info.saved = true;
                        saved_set.insert(SavedMember::ClassInfo);
                    }
                    cs.info.modified = false;
                    for f in &mut cs.fields {
                        if f.modified || f.saved {
                            f.saved = true;
                            saved_set
                                .insert(SavedMember::Field(f.name.clone(), f.descriptor.clone()));
                        }
                        f.modified = false;
                    }
                    for m in &mut cs.methods {
                        if m.modified || m.saved {
                            m.saved = true;
                            saved_set
                                .insert(SavedMember::Method(m.name.clone(), m.descriptor.clone()));
                        }
                        m.modified = false;
                    }
                }
                // 持久化 saved 成员（跨 tab 关闭/重开保留）
                self.editor
                    .saved_members
                    .insert(entry_path.clone(), saved_set);
                if let Some(jar) = &mut self.jar {
                    jar.put(&entry_path, new_bytes);
                }
                log::info!("Saved: {entry_path}");
                self.start_single_decompile(&entry_path, false);
            }
            Err(e) => {
                log::error!("Save failed: {entry_path}: {e}");
                self.toasts.error(t!("editor.save_failed", error = e));
            }
        }
    }

    /// 后台反编译单个 .class 文件
    ///
    /// `write_cache` 为 true 时输出写入缓存目录（首次预览），为 false 时仅返回内存结果（保存后重编译）。
    pub(super) fn start_single_decompile(&mut self, entry_path: &str, write_cache: bool) {
        let jar = match &self.jar {
            Some(j) => j,
            None => return,
        };
        let jar_path = jar.path.clone();
        let hash = if write_cache {
            Some(jar.hash.clone())
        } else {
            None
        };
        let bytes = match jar.get(entry_path) {
            Some(b) => b.to_vec(),
            None => return,
        };
        let class_path = entry_path.to_string();
        let (tx, rx) = mpsc::channel();
        let cp = class_path.clone();
        std::thread::spawn(move || {
            let result = pervius_java_bridge::decompiler::decompile_single_class(
                &bytes,
                &cp,
                &jar_path,
                hash.as_deref(),
            );
            let _ = tx.send(result);
        });
        self.pending_decompiles.push((class_path, rx));
    }

    /// 轮询后台重反编译启动结果
    pub(super) fn check_re_decompile(&mut self) {
        let (name, rx) = match &self.pending_re_decompile {
            Some(v) => v,
            None => return,
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_re_decompile = None;
                return;
            }
        };
        let _name = name.clone();
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
            let result = match self.pending_decompiles[i].1.try_recv() {
                Ok(r) => r,
                Err(mpsc::TryRecvError::Empty) => {
                    i += 1;
                    continue;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending_decompiles.swap_remove(i);
                    continue;
                }
            };
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

    /// 在后台线程打开 JAR 文件
    pub fn open_jar(&mut self, path: &Path) {
        let progress = Arc::new(LoadProgress::new());
        let (tx, rx) = mpsc::channel();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path = path.to_path_buf();
        let p = progress.clone();
        std::thread::spawn(move || {
            let _ = tx.send(JarArchive::open_with_progress(&path, &p));
        });
        // 清除旧状态，进入加载中
        self.file_panel.roots = Vec::new();
        self.file_panel.selected = None;
        self.editor = EditorArea::new();
        self.jar = None;
        self.decompiling = None;
        self.loading = Some(LoadingState {
            name,
            progress,
            receiver: rx,
        });
    }

    /// 打开文件对话框选择 JAR
    pub fn open_jar_dialog(&mut self) {
        let jar_label = t!("layout.java_archive");
        let class_label = t!("layout.class_file");
        let file = rfd::FileDialog::new()
            .add_filter(&*jar_label, &["jar", "zip", "war", "ear"])
            .add_filter(&*class_label, &["class"])
            .pick_file();
        if let Some(path) = file {
            self.open_jar(&path);
        }
    }

    /// 导出反编译源码到用户选择的目录
    ///
    /// 将 Vineflower 缓存目录中的 `.java` / `.kt` 文件复制到目标目录，
    /// 保持原始包结构。
    pub fn export_decompiled(&mut self) {
        let jar = match &self.jar {
            Some(j) => j,
            None => {
                self.toasts.warning(t!("layout.export_no_jar"));
                return;
            }
        };
        if self.decompiling.is_some() {
            self.toasts.warning(t!("layout.decompile_in_progress"));
            return;
        }
        if !decompiler::is_cached(&jar.hash) {
            self.toasts.warning(t!("layout.export_not_decompiled"));
            return;
        }
        let cache = match decompiler::cache_dir(&jar.hash) {
            Ok(d) if d.exists() => d,
            _ => {
                self.toasts.warning(t!("layout.export_not_decompiled"));
                return;
            }
        };
        let dest = match rfd::FileDialog::new().pick_folder() {
            Some(d) => d,
            None => return,
        };
        match copy_sources(&cache, &dest) {
            Ok(count) => {
                let display = dest.to_string_lossy();
                self.toasts
                    .info(t!("layout.export_complete", path = display, count = count));
                log::info!("Exported {count} files to {display}");
            }
            Err(e) => {
                self.toasts.error(t!("layout.export_failed", error = e));
                log::error!("Export failed: {e}");
            }
        }
    }

    /// 从 JAR 条目创建编辑器 tab
    ///
    /// `mem_cached` 为已修改条目的内存反编译缓存，优先于磁盘缓存。
    fn create_tab(
        entry_path: &str,
        bytes: &[u8],
        jar_hash: Option<&str>,
        mem_cached: Option<&CachedSource>,
    ) -> EditorTab {
        let file_name = entry_path.rsplit('/').next().unwrap_or(entry_path);
        let title = file_name.strip_suffix(".class").unwrap_or(file_name);
        if file_name.ends_with(".class") {
            let cached = mem_cached
                .cloned()
                .or_else(|| jar_hash.and_then(|h| decompiler::cached_source(h, entry_path)));
            let lang = match &cached {
                Some(c) if c.is_kotlin => Language::Kotlin,
                _ => Language::Java,
            };
            let mut tab = EditorTab::new_class(title, entry_path, bytes.to_vec(), lang);
            if let Some(c) = cached {
                tab.set_decompiled(c.source, lang, c.line_mapping);
                tab.active_view = ActiveView::Decompiled;
            }
            tab
        } else if Self::is_binary(bytes) {
            EditorTab::new_binary(title, entry_path, bytes.to_vec())
        } else {
            let text = Self::decode_text(bytes);
            let lang = Language::from_filename(file_name);
            EditorTab::new_text(title, entry_path, text, bytes.to_vec(), lang)
        }
    }

    /// 判断字节内容是否为二进制文件（前 8KB 内含 null 字节即视为二进制）
    fn is_binary(bytes: &[u8]) -> bool {
        let check_len = bytes.len().min(8192);
        bytes[..check_len].contains(&0)
    }

    /// 将字节解码为文本（自动检测编码）
    fn decode_text(bytes: &[u8]) -> String {
        // UTF-8 快速路径
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.to_string();
        }
        // 非 UTF-8: 用 chardetng 检测编码后转换
        let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
        detector.feed(bytes, true);
        let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
        let (text, _, _) = encoding.decode(bytes);
        text.into_owned()
    }
}

/// 在资源管理器/Finder 中选中文件
#[cfg(windows)]
fn reveal_in_explorer(path: &Path) {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn SHParseDisplayName(
            name: *const u16,
            ctx: *const c_void,
            pidl: *mut *mut c_void,
            sfgao_in: u32,
            sfgao_out: *mut u32,
        ) -> i32;
        fn SHOpenFolderAndSelectItems(
            dir: *const c_void,
            count: u32,
            items: *const *const c_void,
            flags: u32,
        ) -> i32;
        fn CoTaskMemFree(pv: *mut c_void);
    }

    // 规范化路径分隔符（join 产生的 "/" 混入会导致 SHParseDisplayName 失败）
    let normalized: std::path::PathBuf = path.components().collect();
    let wide: Vec<u16> = normalized
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        let mut pidl: *mut c_void = std::ptr::null_mut();
        if SHParseDisplayName(
            wide.as_ptr(),
            std::ptr::null(),
            &mut pidl,
            0,
            std::ptr::null_mut(),
        ) != 0
        {
            return;
        }
        SHOpenFolderAndSelectItems(pidl, 0, std::ptr::null(), 0);
        CoTaskMemFree(pidl);
    }
}

#[cfg(target_os = "macos")]
fn reveal_in_explorer(path: &Path) {
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
}

#[cfg(not(any(windows, target_os = "macos")))]
fn reveal_in_explorer(path: &Path) {
    // Linux: 用 xdg-open 打开父目录
    if let Some(parent) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }
}

/// 直接打开目录（显示其内容）
#[cfg(windows)]
fn open_directory(path: &Path) {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: *const c_void,
            op: *const u16,
            file: *const u16,
            params: *const u16,
            dir: *const u16,
            show: i32,
        ) -> isize;
    }

    let normalized: std::path::PathBuf = path.components().collect();
    let wide: Vec<u16> = normalized
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let open: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    unsafe {
        ShellExecuteW(
            std::ptr::null(),
            open.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        );
    }
}

#[cfg(not(windows))]
fn open_directory(path: &Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(target_os = "macos"))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// 绘制 JAR 加载进度
pub(super) fn paint_loading(ui: &egui::Ui, rect: egui::Rect, loading: &LoadingState) {
    let current = loading.progress.current.load(Ordering::Relaxed);
    let total = loading.progress.total.load(Ordering::Relaxed);
    let frac = if total > 0 {
        current as f32 / total as f32
    } else {
        0.0
    };
    let painter = ui.painter();
    let cx = rect.center().x;
    let cy = rect.center().y;
    // 标题
    painter.text(
        egui::pos2(cx, cy - 20.0),
        egui::Align2::CENTER_CENTER,
        t!("layout.opening", name = loading.name),
        egui::FontId::proportional(13.0),
        theme::TEXT_SECONDARY,
    );
    // 进度条
    let bar_w = 240.0;
    let bar_h = 4.0;
    let bar_rect = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(bar_w, bar_h));
    painter.rect_filled(bar_rect, 2.0, theme::BG_MEDIUM);
    if frac > 0.0 {
        let fill = egui::Rect::from_min_size(bar_rect.left_top(), egui::vec2(bar_w * frac, bar_h));
        painter.rect_filled(fill, 2.0, theme::VERDIGRIS);
    }
    // 计数
    if total > 0 {
        painter.text(
            egui::pos2(cx, cy + 16.0),
            egui::Align2::CENTER_CENTER,
            format!("{current} / {total}"),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
    }
}

/// 递归复制 `.java` / `.kt` 源码文件到目标目录，保持目录结构
///
/// 跳过 `.complete` 等非源码文件，返回复制的文件数。
fn copy_sources(src: &Path, dest: &Path) -> Result<usize, String> {
    let mut count = 0;
    copy_sources_recursive(src, src, dest, &mut count)?;
    Ok(count)
}

/// 递归遍历源目录，复制匹配的源码文件
fn copy_sources_recursive(
    root: &Path,
    dir: &Path,
    dest: &Path,
    count: &mut usize,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{e}"))?;
        let path = entry.path();
        if path.is_dir() {
            copy_sources_recursive(root, &path, dest, count)?;
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "java" && ext != "kt" {
                continue;
            }
            let rel = path.strip_prefix(root).map_err(|e| format!("{e}"))?;
            let target = dest.join(rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
            }
            std::fs::copy(&path, &target).map_err(|e| format!("{e}"))?;
            *count += 1;
        }
    }
    Ok(())
}
