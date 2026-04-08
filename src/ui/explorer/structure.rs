//! 类大纲视图：字段 / 方法列表
//!
//! @author sky

use crate::shell::{codicon, theme};
use eframe::egui;

/// 渲染类结构大纲（demo 数据）
pub fn render(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("class_structure")
        .show(ui, |ui| {
            ui.add_space(8.0);
            // 类名
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(codicon::SYMBOL_CLASS)
                        .family(codicon::family())
                        .size(14.0)
                        .color(theme::VERDIGRIS),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("public class MinecraftServer")
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY),
                );
            });
            ui.add_space(8.0);
            // FIELDS
            section_label(ui, "FIELDS");
            for name in [
                "serverThread : Thread",
                "levels : List<ServerLevel>",
                "running : boolean",
            ] {
                member_row(ui, codicon::SYMBOL_FIELD, name);
            }
            ui.add_space(8.0);
            // METHODS
            section_label(ui, "METHODS");
            for name in [
                "<init>(Thread)",
                "startServer()",
                "stopServer()",
                "tickServer()",
                "getLevel(ResourceKey)",
            ] {
                member_row(ui, codicon::SYMBOL_METHOD, name);
            }
        });
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(text)
                .size(10.0)
                .color(theme::TEXT_MUTED),
        );
    });
}

fn member_row(ui: &mut egui::Ui, icon: &str, label: &str) {
    let avail_w = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(avail_w, 26.0), egui::Sense::click());
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 4.0, theme::BG_HOVER);
    }
    painter.text(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::new(14.0, codicon::family()),
        theme::VERDIGRIS,
    );
    painter.text(
        egui::pos2(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(11.0),
        theme::TEXT_PRIMARY,
    );
}
