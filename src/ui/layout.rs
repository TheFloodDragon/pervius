//! 主布局：Explorer / Editor 各包裹在独立 Island 内，StatusBar 全宽置底
//!
//! @author sky

mod confirm;
mod handler;
mod island;

use super::editor::EditorArea;
use super::explorer::tree;
use super::explorer::FilePanel;
use super::search::SearchDialog;
use super::settings::SettingsDialog;
use super::status_bar::StatusBar;
use crate::appearance::theme;
use crate::settings::Settings;
use confirm::ConfirmAction;
use eframe::egui;
use egui_keybind::KeyMap;
use egui_notify::Toasts;
use egui_shell::components::SettingsFile;
use pervius_java_bridge::decompiler::{CachedSource, DecompileTask};
use pervius_java_bridge::error::BridgeError;
use pervius_java_bridge::jar::JarArchive;
use std::collections::HashSet;
use std::sync::mpsc;

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
    /// 已反编译的类集合（None = 全部已反编译，Some = 仅集合内的类已完成）
    decompiled_classes: Option<HashSet<String>>,
    /// FPS 叠加层开关（F12）
    show_fps: bool,
    /// 待确认的破坏性动作
    pub pending_confirm: Option<ConfirmAction>,
    /// 单文件反编译结果接收队列（支持并发多个）
    pending_decompiles: Vec<(String, mpsc::Receiver<Result<CachedSource, BridgeError>>)>,
    /// 后台重反编译启动中（清缓存 + start 在子线程）
    pending_re_decompile: Option<(String, mpsc::Receiver<Result<DecompileTask, BridgeError>>)>,
}

struct LayoutRects {
    explorer: egui::Rect,
    editor: egui::Rect,
    status: egui::Rect,
}

impl Layout {
    pub fn new() -> Self {
        let settings = Settings::load();
        let keys = super::keybindings::build_keymap(&settings.keymap);
        Self {
            file_panel: FilePanel::new(),
            editor: EditorArea::new(),
            status_bar: StatusBar::default(),
            search: SearchDialog::new(),
            settings_dialog: SettingsDialog::new(),
            settings,
            toasts: Toasts::default(),
            keys,
            explorer_width: theme::FILE_PANEL_WIDTH,
            explorer_visible: true,
            jar: None,
            loading: None,
            decompiling: None,
            decompiled_classes: None,
            show_fps: false,
            pending_confirm: None,
            pending_decompiles: Vec::new(),
            pending_re_decompile: None,
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui, shell_theme: &egui_shell::ShellTheme) {
        // 拦截窗口关闭请求
        if ui.ctx().input(|i| i.viewport().close_requested()) && self.has_unsaved_changes() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_confirm = Some(ConfirmAction::Close);
        }
        self.handle_dropped_files(ui.ctx());
        self.handle_pending_open();
        self.handle_pending_reveal();
        self.check_loading();
        self.check_decompiling();
        self.check_re_decompile();
        self.check_single_decompile();
        // keybind 录制中跳过快捷键分发，避免录制按键同时触发动作
        if !egui_shell::components::is_recording_keybind(ui.ctx()) {
            let view_before = self.editor.focused_view();
            let mut keys = std::mem::take(&mut self.keys);
            keys.dispatch(ui.ctx(), self);
            self.keys = keys;
            // Tab 切换视图后清除焦点，防止 begin_pass 焦点导航产生的闪烁
            if self.editor.focused_view() != view_before {
                ui.ctx().memory_mut(|m| m.stop_text_input());
            }
            // 快捷键触发的 tab 关闭被 is_modified 拦截
            if let Some(action) = self.editor.blocked_close.take() {
                self.pending_confirm = Some(ConfirmAction::TabClose(action));
            }
        }
        let t = self.explorer_anim_t(ui);
        let rects = Self::compute_rects(ui.max_rect(), self.explorer_width * t, t > 0.0);
        if t > 0.0 {
            self.render_explorer(ui, rects.explorer);
        }
        if self.explorer_visible && t >= 1.0 {
            self.render_resize_handle(ui, &rects);
        }
        self.render_editor(ui, rects.editor);
        if let Some(action) = self.editor.blocked_close.take() {
            if self.pending_confirm.is_none() {
                self.pending_confirm = Some(ConfirmAction::TabClose(action));
            }
        }
        self.sync_explorer_selection();
        self.render_status_bar(ui, rects.status);
        self.search.render(ui.ctx(), shell_theme);
        if let Some(new_settings) = self.settings_dialog.render(ui.ctx(), shell_theme) {
            // keybind 配置变更时重建 KeyMap
            self.keys = super::keybindings::build_keymap(&new_settings.keymap);
            // 语言变更时更新 locale
            if new_settings.language != self.settings.language {
                rust_i18n::set_locale(new_settings.language.code());
            }
            self.settings = new_settings;
            if let Err(e) = self.settings.save() {
                log::warn!("配置保存失败: {e}");
            }
        }
        self.toasts.show(ui.ctx());
        if ui.input(|i| i.key_pressed(egui::Key::F12)) {
            self.show_fps = !self.show_fps;
        }
        if self.show_fps {
            self.render_fps_overlay(ui);
        }
        self.render_confirm(ui.ctx());
    }

    /// 左上角 FPS 叠加层（调试用）
    fn render_fps_overlay(&self, ui: &egui::Ui) {
        let dt = ui.input(|i| i.stable_dt);
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        let text = format!("{fps:.0} fps  {:.1} ms", dt * 1000.0);
        let pos = ui.max_rect().left_top() + egui::vec2(8.0, 8.0);
        let font = egui::FontId::monospace(11.0);
        let painter = ui.painter();
        // 半透明背景
        let galley = painter.layout_no_wrap(text, font, egui::Color32::WHITE);
        let bg = egui::Rect::from_min_size(
            pos - egui::vec2(4.0, 2.0),
            galley.size() + egui::vec2(8.0, 4.0),
        );
        painter.rect_filled(bg, 3.0, egui::Color32::from_black_alpha(160));
        painter.galley(pos, galley, egui::Color32::WHITE);
    }

    /// 获取 explorer 折叠/展开动画插值（同一帧内缓存）
    fn explorer_anim_t(&self, ui: &egui::Ui) -> f32 {
        let anim_id = ui.id().with("explorer_anim");
        self.cached_anim_t(ui, anim_id)
    }

    fn render_explorer(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        island::paint(ui, rect);
        let (tab_modified, jar_modified) = self.split_modified_entries();
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(egui::Id::new("explorer_island"))
                .max_rect(rect),
        );
        child.set_clip_rect(rect);
        self.file_panel.render(
            &mut child,
            &tab_modified,
            &jar_modified,
            self.decompiled_classes.as_ref(),
        );
        island::paint_corner_mask(ui, rect);
    }

    fn render_editor(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        island::paint(ui, rect);
        if let Some(loading) = &self.loading {
            handler::paint_loading(ui, rect, loading);
            ui.ctx().request_repaint();
        } else {
            let jar_modified = self
                .jar
                .as_ref()
                .map(|j| j.modified_paths().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            self.editor.render(
                &mut ui.new_child(
                    egui::UiBuilder::new()
                        .id(egui::Id::new("editor_island"))
                        .max_rect(rect),
                ),
                &jar_modified,
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
        let class_info = self.editor.focused_class_info().map(|s| s.to_string());
        self.status_bar.sync_view(
            self.editor.focused_view(),
            self.editor.focused_is_class(),
            class_info.as_deref(),
        );
        let saved_paths: Vec<String> = self
            .jar
            .as_ref()
            .map(|j| j.modified_paths().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let unsaved_paths = self.editor.unsaved_paths();
        self.status_bar
            .sync_modified_count(saved_paths, unsaved_paths);
        // 同步反编译进度
        if let Some((ref name, _)) = self.pending_re_decompile {
            // 重反编译启动中（后台清缓存）
            self.status_bar.sync_decompile_single(name);
        } else if !self.pending_decompiles.is_empty() {
            // 单文件反编译中（取最后一个的短名）
            let name = &self.pending_decompiles.last().unwrap().0;
            let short = name.rsplit('/').next().unwrap_or(name);
            self.status_bar.sync_decompile_single(short);
        } else {
            let decompile_info = self.decompiling.as_ref().map(|task| {
                let current = task
                    .progress
                    .current
                    .load(std::sync::atomic::Ordering::Relaxed);
                let total = task
                    .progress
                    .total
                    .load(std::sync::atomic::Ordering::Relaxed);
                (task.jar_name.clone(), current, total)
            });
            self.status_bar.sync_decompile(
                decompile_info
                    .as_ref()
                    .map(|(n, c, t)| (n.as_str(), *c, *t)),
            );
        }
        if self.decompiling.is_some()
            || !self.pending_decompiles.is_empty()
            || self.pending_re_decompile.is_some()
        {
            ui.ctx().request_repaint();
        }
        self.status_bar.render(
            &mut ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .id(egui::Id::new("status_bar")),
            ),
        );
        if let Some(v) = self.status_bar.take_view_change() {
            self.editor.set_focused_view(v);
        }
        if let Some(path) = self.status_bar.take_clicked_file() {
            if !self.editor.focus_tab(&path) {
                self.file_panel.pending_open = Some(path);
            }
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
                tree::reveal(&mut self.file_panel.roots, &path);
                self.file_panel.selected = Some(path);
                self.file_panel.scroll_to_selected = true;
            }
        }
    }

    /// 分别收集 tab 级别（橙色）和 JAR 级别（绿色）已修改条目路径（含父级目录）
    fn split_modified_entries(&self) -> (HashSet<String>, HashSet<String>) {
        let mut tab_set = HashSet::new();
        let mut jar_set = HashSet::new();
        for (_, tab) in self.editor.dock_state.iter_all_tabs() {
            if tab.is_modified {
                if let Some(path) = &tab.entry_path {
                    Self::insert_with_parents(&mut tab_set, path);
                }
            }
        }
        if let Some(jar) = &self.jar {
            for path in jar.modified_paths() {
                Self::insert_with_parents(&mut jar_set, path);
            }
        }
        (tab_set, jar_set)
    }

    /// 将路径及其所有父级目录加入集合
    fn insert_with_parents(set: &mut HashSet<String>, path: &str) {
        set.insert(path.to_string());
        let mut p = path;
        while let Some(idx) = p.rfind('/') {
            let parent = &p[..idx + 1];
            if !set.insert(parent.to_string()) {
                break;
            }
            p = &p[..idx];
        }
        set.insert(String::new());
    }

    /// 打开设置对话框
    pub fn open_settings(&mut self) {
        self.settings_dialog.open(&self.settings);
    }
}
