//! 菜单项渲染原语
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;
use egui_keybind::KeyBind;

/// 菜单项：label 靠左，快捷键靠右（从 KeyBind 取标签）
pub fn menu_item(ui: &mut egui::Ui, label: &str, keybind: Option<&KeyBind>) -> bool {
    let shortcut = keybind.map(|k| k.label()).unwrap_or_default();
    menu_item_raw(ui, label, &shortcut)
}

/// 菜单项：label 靠左，快捷键靠右（原始字符串版本，用于系统级快捷键如 Ctrl+C）
pub fn menu_item_raw(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    let label_font = egui::FontId::proportional(12.0);
    let shortcut_font = egui::FontId::proportional(11.0);
    let padding = 8.0;
    let gap = if shortcut.is_empty() { 0.0 } else { 24.0 };
    let height = 22.0;
    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), label_font, theme::TEXT_PRIMARY);
    let shortcut_galley = if !shortcut.is_empty() {
        Some(
            ui.painter()
                .layout_no_wrap(shortcut.to_owned(), shortcut_font, theme::TEXT_MUTED),
        )
    } else {
        None
    };
    let content_w =
        label_galley.size().x + gap + shortcut_galley.as_ref().map_or(0.0, |g| g.size().x);
    let (rect, resp) = ui.allocate_at_least(
        egui::vec2(content_w + padding * 2.0, height),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect, egui::CornerRadius::same(3), theme::BG_HOVER);
    }
    let label_y = rect.center().y - label_galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.left() + padding, label_y),
        label_galley,
        theme::TEXT_PRIMARY,
    );
    if let Some(sg) = shortcut_galley {
        let sy = rect.center().y - sg.size().y / 2.0;
        painter.galley(
            egui::pos2(rect.right() - padding - sg.size().x, sy),
            sg,
            theme::TEXT_MUTED,
        );
    }
    resp.clicked()
}
