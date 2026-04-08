//! 单个标签页渲染
//!
//! @author sky

use crate::shell::{codicon, theme};
use eframe::egui;

/// 标签页数据
pub struct TabInfo {
    pub title: String,
    pub is_active: bool,
    pub is_modified: bool,
}

/// 渲染单个 Tab，返回是否被点击
pub fn render(ui: &mut egui::Ui, tab: &TabInfo) -> bool {
    let text_galley = ui.painter().layout_no_wrap(
        tab.title.clone(),
        egui::FontId::proportional(12.0),
        theme::TEXT_PRIMARY,
    );
    let tab_w = 12.0 + 16.0 + 6.0 + text_galley.size().x + 6.0 + 20.0 + 4.0;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(tab_w, theme::TAB_BAR_HEIGHT),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    let hovered = response.hovered();
    // 背景
    let bg = if tab.is_active {
        theme::BG_MEDIUM
    } else if hovered {
        theme::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 6.0, bg);
    // 底部高亮线
    if tab.is_active {
        let line_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.bottom() - 2.0),
            egui::vec2(rect.width(), 2.0),
        );
        painter.rect_filled(line_rect, 1.0, theme::VERDIGRIS);
    }
    let y = rect.center().y;
    let mut x = rect.left() + 12.0;
    // class 图标
    painter.text(
        egui::pos2(x + 8.0, y),
        egui::Align2::CENTER_CENTER,
        codicon::SYMBOL_CLASS,
        egui::FontId::new(11.0, codicon::family()),
        theme::VERDIGRIS,
    );
    x += 16.0 + 6.0;
    // 标题
    let title_color = if tab.is_active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    painter.text(
        egui::pos2(x, y),
        egui::Align2::LEFT_CENTER,
        &tab.title,
        egui::FontId::proportional(12.0),
        title_color,
    );
    x += text_galley.size().x + 6.0;
    // 修改标记
    if tab.is_modified {
        painter.text(
            egui::pos2(x + 4.0, y),
            egui::Align2::CENTER_CENTER,
            codicon::CIRCLE_FILLED,
            egui::FontId::proportional(8.0),
            theme::ACCENT_ORANGE,
        );
    }
    response.clicked()
}
