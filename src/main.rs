//! Pervius — Java 反编译器 + 字节码编辑器
//!
//! @author sky

mod jar;
mod shell;
mod ui;

use eframe::egui;
use ui::layout::Layout;

fn main() -> eframe::Result {
    env_logger::init();
    let options = shell::ShellOptions {
        title: "Pervius".to_owned(),
        size: [1280.0, 800.0],
        theme: shell::ShellTheme {
            bg: shell::theme::BG_DARK,
            bg_hover: shell::theme::BG_HOVER,
            text_primary: shell::theme::TEXT_PRIMARY,
            text_secondary: shell::theme::TEXT_SECONDARY,
            accent: shell::theme::VERDIGRIS,
            caption_hover: shell::theme::CAPTION_HOVER,
            close_hover: shell::theme::CLOSE_HOVER,
            title_bar_height: shell::theme::TITLE_BAR_HEIGHT,
        },
    };
    shell::run(options, |_cc| {
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
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui::menu::menu_bar(ui, &mut |action| {
            use ui::menu::MenuAction;
            match action {
                MenuAction::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                MenuAction::ToggleExplorer => {
                    self.layout.explorer_visible = !self.layout.explorer_visible
                }
                _ => {}
            }
        });
    }
}
