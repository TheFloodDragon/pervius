//! JAR 文件操作：打开、异步加载、拖拽处理、tab 创建、编码检测、反编译调度
//!
//! @author sky

use super::Layout;
use crate::java::decompiler;
use crate::java::jar::{JarArchive, LoadProgress};
use crate::shell::theme;
use crate::ui::editor::highlight::Language;
use crate::ui::editor::view_toggle::ActiveView;
use crate::ui::editor::{EditorArea, EditorTab};
use crate::ui::explorer::tree;
use eframe::egui;
use egui_window_settings::SettingsFile;
use rust_i18n::t;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};

/// JAR 后台加载状态
pub(super) struct LoadingState {
    pub name: String,
    pub progress: Arc<LoadProgress>,
    pub receiver: mpsc::Receiver<Result<JarArchive, String>>,
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
                    "jar" | "zip" | "war" | "ear" => self.open_jar(path),
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
        let hash = self.jar.as_ref().map(|j| j.hash.as_str());
        let tab = Self::create_tab(&entry_path, &bytes, hash);
        self.editor.open_tab(tab);
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
        match task.receiver.try_recv() {
            Ok(Ok(())) => {
                log::info!("Decompilation complete: {}", task.jar_name);
                let hash = self.jar.as_ref().map(|j| j.hash.as_str());
                self.editor.refresh_class_tabs(hash);
                self.toasts
                    .info(t!("layout.decompile_complete", name = task.jar_name));
                self.decompiling = None;
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
            return;
        }
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
        decompiler::clear_cache(&jar.hash);
        // 需要 clone 出必要数据避免借用冲突
        let path = jar.path.clone();
        let name = jar.name.clone();
        let hash = jar.hash.clone();
        let class_count = jar.class_count();
        match decompiler::start(&path, &name, &hash, class_count) {
            Ok(task) => {
                log::info!("Re-decompiling: {name} ({class_count} classes)");
                self.decompiling = Some(task);
            }
            Err(e) => {
                self.toasts
                    .warning(t!("layout.decompiler_unavailable", error = e));
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

    /// 从 JAR 条目创建编辑器 tab
    fn create_tab(entry_path: &str, bytes: &[u8], jar_hash: Option<&str>) -> EditorTab {
        let file_name = entry_path.rsplit('/').next().unwrap_or(entry_path);
        let title = file_name.strip_suffix(".class").unwrap_or(file_name);
        if file_name.ends_with(".class") {
            let cached = jar_hash.and_then(|h| decompiler::cached_source(h, entry_path));
            let lang = match &cached {
                Some(c) if c.is_kotlin => Language::Kotlin,
                _ => Language::Java,
            };
            let mut tab = EditorTab::new_class(title, entry_path, bytes.to_vec(), lang);
            if let Some(c) = cached {
                tab.decompiled = c.source;
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
        let mut detector = chardetng::EncodingDetector::new();
        detector.feed(bytes, true);
        let encoding = detector.guess(None, true);
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
