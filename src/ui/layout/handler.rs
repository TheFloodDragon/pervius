//! JAR 文件打开、后台加载、拖拽处理、独立文件打开
//!
//! @author sky

use super::Layout;
use crate::appearance::theme;
use crate::ui::editor::EditorArea;
use crate::ui::explorer::tree;
use eframe::egui;
use egui_shell::components::SettingsFile;
use pervius_java_bridge::error::BridgeError;
use pervius_java_bridge::jar::{JarArchive, LoadProgress};
use rust_i18n::t;
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
                    _ => self.open_standalone_file(path),
                }
            }
        }
    }

    /// 检查后台 JAR 加载是否完成
    pub(super) fn check_loading(&mut self) {
        let Some(loading) = &self.loading else { return };
        let result = poll_recv!(loading.receiver,
            miss => return,
            disconnect => { self.loading = None; return; }
        );
        self.loading = None;
        match result {
            Ok(jar) => {
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
            }
            Err(e) => {
                log::error!("Failed to open JAR: {e}");
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

    /// 打开文件对话框选择 JAR 或独立文件
    pub fn open_jar_dialog(&mut self) {
        let jar_label = t!("layout.java_archive");
        let class_label = t!("layout.class_file");
        let file = rfd::FileDialog::new()
            .add_filter(&*jar_label, &["jar", "zip", "war", "ear"])
            .add_filter(&*class_label, &["class"])
            .pick_file();
        if let Some(path) = file {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext.to_ascii_lowercase().as_str() {
                "jar" | "zip" | "war" | "ear" => self.open_jar(&path),
                _ => self.open_standalone_file(&path),
            }
        }
    }

    /// 打开独立文件（非 JAR 条目）为 tab，保存时直接写回磁盘
    pub fn open_standalone_file(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        // 已打开则聚焦
        if self.editor.focus_tab(&path_str) {
            return;
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                log::error!("Failed to read file: {path_str}: {e}");
                self.toasts.error(t!("layout.open_file_failed", error = e));
                return;
            }
        };
        let mut tab = Self::create_tab(&path_str, &bytes, None, None);
        tab.standalone_path = Some(path.to_path_buf());
        self.editor.open_tab(tab);
        // 独立 .class 文件触发反编译（无 JAR 上下文）
        if path_str.ends_with(".class") {
            self.start_single_decompile(&path_str, bytes, false);
        }
    }
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
