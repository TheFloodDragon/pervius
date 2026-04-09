//! 主布局：Explorer / Editor 各包裹在独立 Island 内，StatusBar 全宽置底
//!
//! @author sky

use super::editor::EditorArea;
use super::explorer::FilePanel;
use super::status_bar::StatusBar;
use crate::shell::theme;
use eframe::egui;
use egui_notify::Toasts;

/// 主布局状态
pub struct Layout {
    pub file_panel: FilePanel,
    pub editor: EditorArea,
    pub toasts: Toasts,
}

impl Layout {
    pub fn new() -> Self {
        use super::demo;
        Self {
            file_panel: FilePanel::new(demo::tree_nodes(), demo::search_results()),
            editor: EditorArea::new(demo::editor_tabs()),
            toasts: Toasts::default(),
        }
    }

    /// 在 CentralPanel 内绘制完整布局
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let total = ui.max_rect();
        let mh = theme::ISLAND_MARGIN_H;
        let mv = theme::ISLAND_MARGIN_V;
        let gap = theme::ISLAND_GAP;
        // StatusBar 在最底部全宽（不在 island 内），底部留边距
        let status_bottom_margin = mv;
        let status_top = total.bottom() - theme::STATUS_BAR_HEIGHT - status_bottom_margin;
        // Island 区域：标题栏下方到 StatusBar 上方，四周留 margin
        let island_top = total.top() + mv;
        let island_bottom = status_top - mv;
        let island_left = total.left() + mh;
        let island_right = total.right() - mh;
        // Explorer island
        let explorer_rect = egui::Rect::from_min_max(
            egui::pos2(island_left, island_top),
            egui::pos2(island_left + theme::FILE_PANEL_WIDTH, island_bottom),
        );
        Self::paint_island(ui, explorer_rect);
        self.file_panel
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(explorer_rect)));
        Self::paint_island_corner_mask(ui, explorer_rect);
        // Editor island
        let editor_rect = egui::Rect::from_min_max(
            egui::pos2(explorer_rect.right() + gap, island_top),
            egui::pos2(island_right, island_bottom),
        );
        Self::paint_island(ui, editor_rect);
        self.editor
            .render(&mut ui.new_child(egui::UiBuilder::new().max_rect(editor_rect)));
        Self::paint_island_corner_mask(ui, editor_rect);
        // StatusBar（全宽，不在 island 内，底部留 status_bottom_margin）
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(total.left(), status_top),
            egui::vec2(total.width(), theme::STATUS_BAR_HEIGHT),
        );
        let active_view = self.editor.focused_view();
        let new_view = StatusBar::render(
            &mut ui.new_child(egui::UiBuilder::new().max_rect(status_rect)),
            active_view,
        );
        if let Some(v) = new_view {
            self.editor.set_focused_view(v);
        }
        // Toast 通知
        self.toasts.show(ui.ctx());
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
            // 圆弧采样点（从 start_angle 到 start_angle + PI/2）
            for i in 0..=segments {
                let t = *start_angle + quarter * (i as f32 / segments as f32);
                let p = egui::pos2(center.x + r * t.cos(), center.y + r * t.sin());
                mesh.colored_vertex(p, color);
            }
            // 三角扇形：corner → arc[i] → arc[i+1]
            for i in 0..segments {
                let a = corner_idx + 1 + i as u32;
                mesh.add_triangle(corner_idx, a, a + 1);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}
