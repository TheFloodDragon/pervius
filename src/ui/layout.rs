//! 主布局：Explorer / Editor 各包裹在独立 Island 内，StatusBar 全宽置底
//!
//! @author sky

mod handler;
mod island;

use super::editor::EditorArea;
use super::explorer::FilePanel;
use super::search::SearchDialog;
use super::settings::SettingsDialog;
use super::status_bar::StatusBar;
use crate::decompiler::DecompileTask;
use crate::jar::JarArchive;
use crate::settings::Settings;
use crate::shell::theme;
use eframe::egui;
use egui_keybind::KeyMap;
use egui_notify::Toasts;

/// Explorer 面板最小宽度
const EXPLORER_MIN_WIDTH: f32 = 160.0;
/// Explorer 面板最大宽度
const EXPLORER_MAX_WIDTH: f32 = 600.0;

/// Explorer 折叠/展开动画时长（秒）
const EXPLORER_ANIM_DURATION: f32 = 0.08;

/// 主布局状态
pub struct Layout {
    pub file_panel: FilePanel,
    pub editor: EditorArea,
    pub status_bar: StatusBar,
    pub search: SearchDialog,
    pub settings_dialog: SettingsDialog,
    pub settings: Settings,
    pub toasts: Toasts,
    keys: KeyMap<Self>,
    /// Explorer 面板当前宽度（可拖拽调整）
    explorer_width: f32,
    /// Explorer 面板是否可见
    pub explorer_visible: bool,
    /// 当前打开的 JAR 归档
    jar: Option<JarArchive>,
    /// 后台加载中的 JAR
    loading: Option<handler::LoadingState>,
    /// 后台反编译任务
    decompiling: Option<DecompileTask>,
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
            settings_dialog: SettingsDialog::new(),
            settings: Settings::load(),
            toasts: Toasts::default(),
            keys: super::keybindings::build_keymap(),
            explorer_width: theme::FILE_PANEL_WIDTH,
            explorer_visible: true,
            jar: None,
            loading: None,
            decompiling: None,
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.handle_dropped_files(ui.ctx());
        self.handle_pending_open();
        self.handle_pending_reveal();
        self.check_loading();
        self.check_decompiling();
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
        self.sync_explorer_selection();
        self.render_status_bar(ui, rects.status);
        self.search.render(ui.ctx());
        if let Some(new_settings) = self.settings_dialog.render(ui.ctx()) {
            self.settings = new_settings;
            if let Err(e) = self.settings.save() {
                log::warn!("配置保存失败: {e}");
            }
        }
        self.toasts.show(ui.ctx());
    }

    /// 获取 explorer 折叠/展开动画插值（同一帧内缓存）
    fn explorer_anim_t(&self, ui: &egui::Ui) -> f32 {
        let anim_id = ui.id().with("explorer_anim");
        self.cached_anim_t(ui, anim_id)
    }

    fn render_explorer(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        island::paint(ui, rect);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(egui::Id::new("explorer_island"))
                .max_rect(rect),
        );
        child.set_clip_rect(rect);
        self.file_panel.render(&mut child);
        island::paint_corner_mask(ui, rect);
    }

    fn render_editor(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        island::paint(ui, rect);
        if let Some(loading) = &self.loading {
            handler::paint_loading(ui, rect, loading);
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
        island::paint_corner_mask(ui, rect);
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
        // 同步反编译进度
        let decompile_info = self.decompiling.as_ref().map(|task| {
            let current = task
                .progress
                .current
                .load(std::sync::atomic::Ordering::Relaxed);
            let total = task
                .progress
                .total
                .load(std::sync::atomic::Ordering::Relaxed);
            (task.jar_name.as_str(), current, total)
        });
        self.status_bar.sync_decompile(decompile_info);
        if self.decompiling.is_some() {
            ui.ctx().request_repaint();
        }
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

    /// 编辑器聚焦 tab 变化时同步 explorer 选中状态
    fn sync_explorer_selection(&mut self) {
        if let Some(path) = self.editor.focused_entry_path() {
            if self.file_panel.selected.as_ref() != Some(&path) {
                self.file_panel.selected = Some(path);
            }
        }
    }

    /// 打开设置对话框
    pub fn open_settings(&mut self) {
        self.settings_dialog.open(&self.settings);
    }
}
