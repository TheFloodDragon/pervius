//! Pervius — Java 反编译器 + 字节码编辑器
//!
//! @author sky

mod shell;
mod ui;

use eframe::egui;
use ui::layout::Layout;

fn main() -> eframe::Result {
    shell::run(shell::ShellOptions::default(), |_cc| {
        Box::new(PervApp {
            layout: Layout::new(),
        })
    })
}

struct PervApp {
    layout: Layout,
}

impl shell::AppContent for PervApp {
    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        self.layout.render(ui);
    }
}
