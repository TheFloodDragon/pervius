//! Pervius — Java 反编译器 + 字节码编辑器
//!
//! @author sky

mod shell;

use eframe::egui;

fn main() -> eframe::Result {
    shell::run(shell::ShellOptions::default(), |_cc| Box::new(PervApp))
}

struct PervApp;

impl shell::AppContent for PervApp {
    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(shell::codicon::FOLDER_OPENED)
                    .family(shell::codicon::family())
                    .size(18.0)
                    .color(shell::theme::VERDIGRIS),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("egui demo \u{2014} Codicon \u{5df2}\u{96c6}\u{6210}")
                    .size(18.0)
                    .color(shell::theme::VERDIGRIS),
            );
        });
    }
}
