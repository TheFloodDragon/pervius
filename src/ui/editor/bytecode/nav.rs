//! 字节码面板左侧成员导航栏
//!
//! @author sky

use pervius_java_bridge::bytecode::descriptor::{short_class_name, short_descriptor, short_params};
use crate::appearance::{codicon, theme};
use crate::ui::editor::tab::BytecodeSelection;
use eframe::egui;
use egui_animation::Anim;
use pervius_java_bridge::class_structure::ClassStructure;

/// 导航项行高
const NAV_ROW_HEIGHT: f32 = 24.0;
/// section label 行高
const SECTION_LABEL_HEIGHT: f32 = 28.0;

/// 成员修改状态 → 颜色：未保存→橙色，已保存→绿色，未修改→无
fn member_color(modified: bool, saved: bool) -> Option<egui::Color32> {
    if modified {
        Some(theme::ACCENT_ORANGE)
    } else if saved {
        Some(theme::ACCENT_GREEN)
    } else {
        None
    }
}

pub fn render_nav(
    ui: &mut egui::Ui,
    cs: &ClassStructure,
    selection: BytecodeSelection,
    field_count: usize,
    method_count: usize,
) -> Option<BytecodeSelection> {
    let mut new_selection = None;
    render_section_label(ui, "CLASS INFO");
    if render_nav_item(
        ui,
        codicon::SYMBOL_CLASS,
        &short_class_name(&cs.info.name),
        "",
        selection == BytecodeSelection::ClassInfo,
        member_color(cs.info.modified, cs.info.saved),
    ) {
        new_selection = Some(BytecodeSelection::ClassInfo);
    }
    if !cs.fields.is_empty() {
        ui.add_space(4.0);
        render_section_label(ui, &format!("FIELDS ({field_count})"));
        for (i, field) in cs.fields.iter().enumerate() {
            let selected = selection == BytecodeSelection::Field(i);
            let type_hint = short_descriptor(&field.descriptor);
            if render_nav_item(
                ui,
                codicon::SYMBOL_FIELD,
                &field.name,
                &type_hint,
                selected,
                member_color(field.modified, field.saved),
            ) {
                new_selection = Some(BytecodeSelection::Field(i));
            }
        }
    }
    if !cs.methods.is_empty() {
        ui.add_space(4.0);
        render_section_label(ui, &format!("METHODS ({method_count})"));
        for (i, method) in cs.methods.iter().enumerate() {
            let selected = selection == BytecodeSelection::Method(i);
            let params = short_params(&method.descriptor);
            let label = format!("{}{params}", method.name);
            if render_nav_item(
                ui,
                codicon::SYMBOL_METHOD,
                &label,
                "",
                selected,
                member_color(method.modified, method.saved),
            ) {
                new_selection = Some(BytecodeSelection::Method(i));
            }
        }
    }
    new_selection
}

fn render_section_label(ui: &mut egui::Ui, text: &str) {
    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail_w, SECTION_LABEL_HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(10.0),
        theme::TEXT_MUTED,
    );
}

fn render_nav_item(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    suffix: &str,
    selected: bool,
    mod_color: Option<egui::Color32>,
) -> bool {
    let avail_w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(avail_w, NAV_ROW_HEIGHT), egui::Sense::click());
    let painter = ui.painter();
    // 用 resp.id 作为动画 key，避免同名方法（如重载 <init>()）共享动画状态
    let anim = Anim::new(ui, 0.1).with(resp.id);
    let bg = anim.select_bg(
        selected,
        resp.hovered(),
        resp.clicked(),
        theme::BG_HOVER,
        theme::BG_LIGHT,
    );
    if bg.a() > 0 {
        painter.rect_filled(rect, 0.0, bg);
    }
    if selected {
        painter.rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height())),
            0.0,
            theme::VERDIGRIS,
        );
    }
    let mid_y = rect.center().y;
    let icon_x = rect.left() + 12.0;
    let text_x = icon_x + 18.0;
    painter.text(
        egui::pos2(icon_x, mid_y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::new(12.0, codicon::family()),
        if selected {
            theme::VERDIGRIS
        } else {
            theme::TEXT_MUTED
        },
    );
    let target_label = if let Some(c) = mod_color {
        c
    } else if selected {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    let label_color = Anim::new(ui, 0.35)
        .with(resp.id)
        .color("label", target_label);
    let suffix_galley = painter.layout_no_wrap(
        suffix.to_string(),
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    let suffix_w = suffix_galley.size().x;
    let suffix_pad = if suffix.is_empty() { 0.0 } else { 12.0 };
    let label_galley = painter.layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.0),
        label_color,
    );
    let label_clip = egui::Rect::from_min_max(
        egui::pos2(text_x, rect.top()),
        egui::pos2(rect.right() - suffix_w - suffix_pad, rect.bottom()),
    );
    painter.with_clip_rect(label_clip).galley(
        egui::pos2(text_x, mid_y - label_galley.size().y / 2.0),
        label_galley,
        label_color,
    );
    if !suffix.is_empty() {
        let clip = egui::Rect::from_min_max(
            egui::pos2(text_x, rect.top()),
            egui::pos2(rect.right() - 4.0, rect.bottom()),
        );
        painter.with_clip_rect(clip).galley(
            egui::pos2(
                rect.right() - 4.0 - suffix_w,
                mid_y - suffix_galley.size().y / 2.0,
            ),
            suffix_galley,
            theme::TEXT_MUTED,
        );
    }
    resp.clicked()
}
