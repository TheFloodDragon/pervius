//! 主布局：Explorer / Editor 各包裹在独立 Island 内，StatusBar 全宽置底
//!
//! @author sky

use super::editor::highlight::Language;
use super::editor::EditorArea;
use super::editor::EditorTab;
use super::explorer::FilePanel;
use super::search::SearchDialog;
use super::status_bar::StatusBar;
use crate::jar::{JarArchive, LoadProgress};
use crate::shell::theme;
use eframe::egui;
use egui_keybind::KeyMap;
use egui_notify::Toasts;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};

/// Explorer 面板最小宽度
const EXPLORER_MIN_WIDTH: f32 = 160.0;
/// Explorer 面板最大宽度
const EXPLORER_MAX_WIDTH: f32 = 600.0;

/// Explorer 折叠/展开动画时长（秒）
const EXPLORER_ANIM_DURATION: f32 = 0.08;

/// JAR 后台加载状态
struct LoadingState {
    name: String,
    progress: Arc<LoadProgress>,
    receiver: mpsc::Receiver<Result<JarArchive, String>>,
}

/// 主布局状态
pub struct Layout {
    pub file_panel: FilePanel,
    pub editor: EditorArea,
    pub status_bar: StatusBar,
    pub search: SearchDialog,
    pub toasts: Toasts,
    keys: KeyMap<Self>,
    /// Explorer 面板当前宽度（可拖拽调整）
    explorer_width: f32,
    /// Explorer 面板是否可见
    pub explorer_visible: bool,
    /// 当前打开的 JAR 归档
    jar: Option<JarArchive>,
    /// 后台加载中的 JAR
    loading: Option<LoadingState>,
}

struct LayoutRects {
    explorer: egui::Rect,
    editor: egui::Rect,
    status: egui::Rect,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            file_panel: FilePanel::new(),
            editor: EditorArea::new(),
            status_bar: StatusBar::default(),
            search: SearchDialog::new(),
            toasts: Toasts::default(),
            keys: super::keybindings::build_keymap(),
            explorer_width: theme::FILE_PANEL_WIDTH,
            explorer_visible: true,
            jar: None,
            loading: None,
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.handle_dropped_files(ui.ctx());
        self.handle_pending_open();
        self.check_loading();
        let mut keys = std::mem::take(&mut self.keys);
        keys.dispatch(ui.ctx(), self);
        self.keys = keys;
        let t = self.explorer_anim_t(ui);
        let rects = Self::compute_rects(ui.max_rect(), self.explorer_width * t, t > 0.0);
        if t > 0.0 {
            self.render_explorer(ui, rects.explorer);
        }
        if self.explorer_visible && t >= 1.0 {
            self.render_resize_handle(ui, &rects);
        }
        self.render_editor(ui, rects.editor);
        self.render_status_bar(ui, rects.status);
        self.search.render(ui.ctx());
        self.toasts.show(ui.ctx());
    }

    /// 处理拖拽到窗口的文件
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
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
    fn handle_pending_open(&mut self) {
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
        let tab = Self::create_tab(&entry_path, &bytes);
        self.editor.open_tab(tab);
    }

    /// 检查后台 JAR 加载是否完成
    fn check_loading(&mut self) {
        let loading = match &self.loading {
            Some(l) => l,
            None => return,
        };
        match loading.receiver.try_recv() {
            Ok(Ok(jar)) => {
                use super::explorer::tree;
                let paths = jar.paths();
                self.file_panel.roots = tree::build_tree(&jar.name, &paths);
                self.file_panel.selected = None;
                self.file_panel.filter.clear();
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
        self.loading = Some(LoadingState {
            name,
            progress,
            receiver: rx,
        });
    }

    /// 打开文件对话框选择 JAR
    pub fn open_jar_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Java Archive", &["jar", "zip", "war", "ear"])
            .add_filter("Class File", &["class"])
            .pick_file();
        if let Some(path) = file {
            self.open_jar(&path);
        }
    }

    /// 从 JAR 条目创建编辑器 tab
    fn create_tab(entry_path: &str, bytes: &[u8]) -> EditorTab {
        let file_name = entry_path.rsplit('/').next().unwrap_or(entry_path);
        let title = file_name.strip_suffix(".class").unwrap_or(file_name);
        if file_name.ends_with(".class") {
            EditorTab::new_class(title, entry_path, bytes.to_vec())
        } else {
            let text = Self::decode_text(bytes);
            let lang = Language::from_filename(file_name);
            EditorTab::new_text(title, entry_path, text, bytes.to_vec(), lang)
        }
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

    /// 获取 explorer 折叠/展开动画插值（同一帧内缓存）
    fn explorer_anim_t(&self, ui: &egui::Ui) -> f32 {
        let anim_id = ui.id().with("explorer_anim");
        self.cached_anim_t(ui, anim_id)
    }

    fn render_explorer(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        Self::paint_island(ui, rect);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(egui::Id::new("explorer_island"))
                .max_rect(rect),
        );
        child.set_clip_rect(rect);
        self.file_panel.render(&mut child);
        Self::paint_island_corner_mask(ui, rect);
    }

    fn render_editor(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        Self::paint_island(ui, rect);
        if let Some(loading) = &self.loading {
            paint_loading(ui, rect, loading);
            ui.ctx().request_repaint();
        } else {
            self.editor.render(
                &mut ui.new_child(
                    egui::UiBuilder::new()
                        .id(egui::Id::new("editor_island"))
                        .max_rect(rect),
                ),
            );
        }
        Self::paint_island_corner_mask(ui, rect);
    }

    /// 在 explorer 和 editor 之间的 gap 区域绘制可拖拽的 resize 手柄
    fn render_resize_handle(&mut self, ui: &mut egui::Ui, rects: &LayoutRects) {
        let handle_rect = egui::Rect::from_min_max(
            egui::pos2(rects.explorer.right() - 2.0, rects.explorer.top()),
            egui::pos2(rects.editor.left() + 2.0, rects.explorer.bottom()),
        );
        let id = ui.id().with("explorer_resize");
        let sense = ui.interact(handle_rect, id, egui::Sense::drag());
        if sense.dragged() {
            let delta = sense.drag_delta().x;
            self.explorer_width =
                (self.explorer_width + delta).clamp(EXPLORER_MIN_WIDTH, EXPLORER_MAX_WIDTH);
        }
        if sense.dragged() || sense.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        }
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        self.status_bar.sync_view(self.editor.focused_view());
        self.status_bar
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(rect)));
        if let Some(v) = self.status_bar.take_view_change() {
            self.editor.set_focused_view(v);
        }
    }

    /// 从总 rect 计算各区域的 rect
    fn compute_rects(
        total: egui::Rect,
        explorer_width: f32,
        explorer_visible: bool,
    ) -> LayoutRects {
        let mh = theme::ISLAND_MARGIN_H;
        let mv = theme::ISLAND_MARGIN_V;
        let status_top = total.bottom() - theme::STATUS_BAR_HEIGHT - mv;
        let island_top = total.top() + mv * 0.5;
        let island_bottom = status_top - mv;
        let island_left = total.left() + mh;
        let island_right = total.right() - mh;
        let (explorer, editor_left) = if explorer_visible {
            let explorer = egui::Rect::from_min_max(
                egui::pos2(island_left, island_top),
                egui::pos2(island_left + explorer_width, island_bottom),
            );
            (explorer, explorer.right() + theme::ISLAND_GAP)
        } else {
            (egui::Rect::NOTHING, island_left)
        };
        let editor = egui::Rect::from_min_max(
            egui::pos2(editor_left, island_top),
            egui::pos2(island_right, island_bottom),
        );
        let status = egui::Rect::from_min_size(
            egui::pos2(total.left(), status_top),
            egui::vec2(total.width(), theme::STATUS_BAR_HEIGHT),
        );
        LayoutRects {
            explorer,
            editor,
            status,
        }
    }

    /// 获取动画插值，同一帧内缓存结果避免多 pass 间抖动
    fn cached_anim_t(&self, ui: &egui::Ui, anim_id: egui::Id) -> f32 {
        let ctx = ui.ctx();
        let frame = ctx.cumulative_frame_nr();
        let cache_id = anim_id.with("frame_cache");
        let cached: Option<(u64, f32)> = ctx.data(|d| d.get_temp(cache_id));
        if let Some((f, t)) = cached {
            if f == frame {
                return t;
            }
        }
        let t = ctx.animate_bool_with_time(anim_id, self.explorer_visible, EXPLORER_ANIM_DURATION);
        ctx.data_mut(|d| d.insert_temp::<(u64, f32)>(cache_id, (frame, t)));
        t
    }

    /// 绘制 island 圆角背景（深色，与窗口底色 BG_DARK 形成对比）
    fn paint_island(ui: &egui::Ui, rect: egui::Rect) {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(theme::ISLAND_RADIUS),
            theme::BG_DARKEST,
        );
    }

    /// 在 island 四角绘制窗口底色遮罩，裁剪溢出的方角内容
    ///
    /// 每个角是一个 r×r 正方形，内部挖去四分之一圆弧，剩余区域填充窗口底色。
    /// 通过 mesh 三角扇形实现：圆心 + 圆弧上若干采样点 + 两条直边端点。
    fn paint_island_corner_mask(ui: &egui::Ui, rect: egui::Rect) {
        let r = theme::ISLAND_RADIUS as f32;
        let color = theme::BG_DARK;
        let painter = ui.painter();
        // 四个角：(角落坐标, 圆心坐标, 起始角度)
        let corners = [
            (
                rect.left_top(),
                egui::pos2(rect.left() + r, rect.top() + r),
                std::f32::consts::PI,
            ),
            (
                egui::pos2(rect.right(), rect.top()),
                egui::pos2(rect.right() - r, rect.top() + r),
                -std::f32::consts::FRAC_PI_2,
            ),
            (
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.right() - r, rect.bottom() - r),
                0.0,
            ),
            (
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.left() + r, rect.bottom() - r),
                std::f32::consts::FRAC_PI_2,
            ),
        ];
        let segments = 8;
        let quarter = std::f32::consts::FRAC_PI_2;
        for (corner, center, start_angle) in &corners {
            let mut mesh = egui::Mesh::default();
            let corner_idx = mesh.vertices.len() as u32;
            mesh.colored_vertex(*corner, color);
            for i in 0..=segments {
                let t = *start_angle + quarter * (i as f32 / segments as f32);
                let p = egui::pos2(center.x + r * t.cos(), center.y + r * t.sin());
                mesh.colored_vertex(p, color);
            }
            for i in 0..segments {
                let a = corner_idx + 1 + i as u32;
                mesh.add_triangle(corner_idx, a, a + 1);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}

/// 绘制 JAR 加载进度
fn paint_loading(ui: &egui::Ui, rect: egui::Rect, loading: &LoadingState) {
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
        format!("Opening {}...", loading.name),
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
