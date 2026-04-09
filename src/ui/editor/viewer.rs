//! TabViewer 实现：定义每个 Tab 的标题和内容渲染
//!
//! @author sky

use super::render;
use super::tab::EditorTab;
use super::view_toggle::ActiveView;
use crate::shell::{codicon, theme};
use eframe::egui;
use egui_dock::TabViewer;

pub struct EditorTabViewer;

impl TabViewer for EditorTabViewer {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let mut job = egui::text::LayoutJob::default();
        // 左侧额外 4px 间距（egui_dock 硬编码 x_spacing=8，补到 12）
        job.append("", 4.0, egui::TextFormat::default());
        // class 图标
        job.append(
            codicon::SYMBOL_CLASS,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(11.0, codicon::family()),
                color: theme::VERDIGRIS,
                ..Default::default()
            },
        );
        job.append(" ", 0.0, egui::TextFormat::default());
        // 标题文字（用 PLACEHOLDER 让 egui_dock 的 text_color 控制颜色）
        job.append(
            &tab.title,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(12.0),
                color: egui::Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        // 修改标记
        if tab.is_modified {
            job.append(" ", 0.0, egui::TextFormat::default());
            job.append(
                codicon::CIRCLE_FILLED,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(6.0),
                    color: theme::ACCENT_ORANGE,
                    ..Default::default()
                },
            );
        }
        // 右侧额外 4px 间距
        job.append("", 4.0, egui::TextFormat::default());
        job.into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        match tab.active_view {
            ActiveView::Decompiled => render::render_decompiled(ui, tab),
            ActiveView::Bytecode => render::render_bytecode(ui, tab),
            ActiveView::Hex => render::render_hex(ui, tab),
        }
    }

    /// 禁止拖拽 tab 脱离为独立窗口（样式难以维护），保留拖拽分屏
    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
