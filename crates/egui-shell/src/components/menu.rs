//! 菜单项渲染原语
//!
//! 提供 `menu_item` / `menu_item_raw` / `menu_submenu` 三个函数，
//! 通过 `MenuTheme` 接收配色，与具体应用解耦。
//!
//! @author sky

use crate::codicon;
use eframe::egui;
use egui_keybind::KeyBind;

/// 菜单配色
#[derive(Clone)]
pub struct MenuTheme {
    /// 主文字色（label）
    pub text_primary: egui::Color32,
    /// 暗淡文字色（快捷键）
    pub text_muted: egui::Color32,
    /// hover 背景色
    pub bg_hover: egui::Color32,
}

/// 菜单项内容最小宽度上下文 key
///
/// `menu_submenu` 在渲染子菜单前将上一帧的内容宽度写入此 key，
/// `menu_item_raw` 读取后用作分配宽度下限，使所有项等宽。
const MENU_FILL_WIDTH: &str = "__menu_fill_w";

/// 菜单项：label 靠左，快捷键靠右（从 KeyBind 取标签）
pub fn menu_item(
    ui: &mut egui::Ui,
    theme: &MenuTheme,
    label: &str,
    keybind: Option<&KeyBind>,
) -> bool {
    let shortcut = keybind.map(|k| k.label()).unwrap_or_default();
    menu_item_raw(ui, theme, label, &shortcut)
}

/// 菜单项：label 靠左，快捷键靠右（原始字符串版本，用于系统级快捷键如 Ctrl+C）
pub fn menu_item_raw(ui: &mut egui::Ui, theme: &MenuTheme, label: &str, shortcut: &str) -> bool {
    let label_font = egui::FontId::proportional(12.0);
    let shortcut_font = egui::FontId::proportional(11.0);
    let padding = 8.0;
    let gap = if shortcut.is_empty() { 0.0 } else { 24.0 };
    let height = 22.0;
    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), label_font, theme.text_primary);
    let shortcut_galley = if !shortcut.is_empty() {
        Some(
            ui.painter()
                .layout_no_wrap(shortcut.to_owned(), shortcut_font, theme.text_muted),
        )
    } else {
        None
    };
    let content_w =
        label_galley.size().x + gap + shortcut_galley.as_ref().map_or(0.0, |g| g.size().x);
    let content_min = content_w + padding * 2.0;
    // 如果上层 menu_submenu 设置了填充宽度，用它作为下限（让所有项等宽）
    let fill: f32 = ui
        .ctx()
        .data(|d| d.get_temp(egui::Id::new(MENU_FILL_WIDTH)).unwrap_or(0.0));
    let desired_w = content_min.max(fill);
    let (rect, resp) = ui.allocate_at_least(egui::vec2(desired_w, height), egui::Sense::click());
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect, egui::CornerRadius::same(3), theme.bg_hover);
    }
    let label_y = rect.center().y - label_galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.left() + padding, label_y),
        label_galley,
        theme.text_primary,
    );
    if let Some(sg) = shortcut_galley {
        let sy = rect.center().y - sg.size().y / 2.0;
        painter.galley(
            egui::pos2(rect.right() - padding - sg.size().x, sy),
            sg,
            theme.text_muted,
        );
    }
    resp.clicked()
}

/// 带子菜单的菜单项：label 靠左，右侧 chevron，hover 时展开子菜单
pub fn menu_submenu(
    ui: &mut egui::Ui,
    theme: &MenuTheme,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let label_font = egui::FontId::proportional(12.0);
    let chevron_font = egui::FontId::new(10.0, codicon::family());
    let padding = 8.0;
    let height = 22.0;
    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), label_font, theme.text_primary);
    let chevron_galley = ui.painter().layout_no_wrap(
        codicon::CHEVRON_RIGHT.to_owned(),
        chevron_font,
        theme.text_muted,
    );
    let content_w = label_galley.size().x + 24.0 + chevron_galley.size().x;
    let desired_w = content_w + padding * 2.0;
    let (rect, resp) = ui.allocate_at_least(egui::vec2(desired_w, height), egui::Sense::hover());
    let painter = ui.painter();
    if resp.hovered() {
        painter.rect_filled(rect, egui::CornerRadius::same(3), theme.bg_hover);
    }
    let label_y = rect.center().y - label_galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.left() + padding, label_y),
        label_galley,
        theme.text_primary,
    );
    let cy = rect.center().y - chevron_galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.right() - padding - chevron_galley.size().x, cy),
        chevron_galley,
        theme.text_muted,
    );
    // 子菜单弹出
    let sub_id = ui.id().with(label).with("_sub");
    let was_open: bool = ui.data(|d| d.get_temp(sub_id).unwrap_or(false));
    if resp.hovered() || was_open {
        let width_id = sub_id.with("_w");
        let cached_width: f32 = ui.ctx().data(|d| d.get_temp(width_id).unwrap_or(0.0));
        let fill_key = egui::Id::new(MENU_FILL_WIDTH);
        ui.ctx().data_mut(|d| d.insert_temp(fill_key, cached_width));
        let popup_pos = egui::pos2(rect.right(), rect.top());
        let frame = egui::Frame::popup(ui.style());
        let area_resp = egui::Area::new(sub_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_pos)
            .show(ui.ctx(), |ui| {
                frame.show(ui, |ui| {
                    ui.style_mut().visuals.widgets.hovered.bg_fill = theme.bg_hover;
                    ui.set_min_width(cached_width);
                    add_contents(ui);
                    let w = ui.min_rect().width();
                    if (w - cached_width).abs() > 1.0 {
                        ui.ctx().request_repaint();
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(width_id, w));
                });
            });
        // 清除全局 key，不影响其他菜单
        ui.ctx().data_mut(|d| d.insert_temp(fill_key, 0.0f32));
        // trigger 或 popup 区域内有 hover 就保持打开
        let popup_rect = area_resp.response.rect;
        let still_hovering = ui.ctx().input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|p| [rect, popup_rect].iter().any(|r| r.expand(4.0).contains(p)))
        });
        ui.data_mut(|d| d.insert_temp(sub_id, still_hovering));
    } else {
        ui.data_mut(|d| d.insert_temp::<bool>(sub_id, false));
    }
}
