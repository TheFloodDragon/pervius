//! egui_dock 主题样式配置
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;
use egui_dock::Style as DockStyle;

/// 构建匹配主题的 dock 样式
pub fn build(egui_style: &egui::Style) -> DockStyle {
    let mut style = DockStyle::from_egui(egui_style);
    // Tab 栏背景（与行号栏一致，视觉上贯通）
    style.tab_bar.bg_fill = theme::BG_GUTTER;
    // 活跃 Tab 不显示底部分隔线（视觉上与内容区融合）
    style.tab.hline_below_active_tab_name = false;
    // 活跃 Tab
    style.tab.active.bg_fill = theme::BG_MEDIUM;
    style.tab.active.text_color = theme::TEXT_PRIMARY;
    style.tab.active.outline_color = egui::Color32::TRANSPARENT;
    // 非活跃 Tab
    style.tab.inactive.bg_fill = theme::BG_GUTTER;
    style.tab.inactive.text_color = theme::TEXT_SECONDARY;
    style.tab.inactive.outline_color = egui::Color32::TRANSPARENT;
    // 获得焦点的 Tab（与非活跃一致）
    style.tab.focused.bg_fill = theme::BG_GUTTER;
    style.tab.focused.text_color = theme::TEXT_PRIMARY;
    style.tab.focused.outline_color = egui::Color32::TRANSPARENT;
    // Hovered / keyboard focus 状态也去掉描边
    style.tab.hovered.outline_color = egui::Color32::TRANSPARENT;
    style.tab.active_with_kb_focus.outline_color = egui::Color32::TRANSPARENT;
    style.tab.inactive_with_kb_focus.outline_color = egui::Color32::TRANSPARENT;
    style.tab.focused_with_kb_focus.outline_color = egui::Color32::TRANSPARENT;
    // tab body 边框和内边距清零，背景透明让底层 gutter 背景透出
    style.tab.tab_body.stroke = egui::Stroke::NONE;
    style.tab.tab_body.inner_margin = egui::Margin::ZERO;
    style.tab.tab_body.bg_fill = egui::Color32::TRANSPARENT;
    // 分隔线
    style.separator.color_idle = theme::BORDER;
    style.separator.color_hovered = theme::VERDIGRIS;
    style.separator.color_dragged = theme::VERDIGRIS;
    style
}
