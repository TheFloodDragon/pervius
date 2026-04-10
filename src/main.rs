//! Pervius — Java 反编译器 + 字节码编辑器
//!
//! @author sky

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

rust_i18n::i18n!("locales", fallback = "en");

mod app;
mod appearance;
mod settings;
mod task;
mod ui;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    // 从持久化配置中读取语言并设置 locale
    let saved_settings = settings::Settings::load_for_locale();
    rust_i18n::set_locale(saved_settings.language.code());
    let options = appearance::ShellOptions {
        title: "Pervius".to_owned(),
        size: [1280.0, 800.0],
        theme: appearance::ShellTheme {
            bg: appearance::theme::BG_DARK,
            bg_hover: appearance::theme::BG_HOVER,
            bg_active: appearance::theme::BG_LIGHT,
            text_primary: appearance::theme::TEXT_PRIMARY,
            text_secondary: appearance::theme::TEXT_SECONDARY,
            text_muted: appearance::theme::TEXT_MUTED,
            accent: appearance::theme::VERDIGRIS,
            separator: appearance::theme::BORDER,
            icon_font: eframe::egui::FontFamily::Name("codicon".into()),
            caption_hover: appearance::theme::CAPTION_HOVER,
            close_hover: appearance::theme::CLOSE_HOVER,
            title_bar_height: appearance::theme::TITLE_BAR_HEIGHT,
            window: appearance::theme::window_config(),
        },
    };
    appearance::run(options, |_cc| Box::new(PervApp { app: App::new() }))
}

struct PervApp {
    app: App,
}

impl appearance::AppContent for PervApp {
    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context, theme: &appearance::ShellTheme) {
        self.app.render(ui, theme);
    }
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        ui::menu::menu_bar(ui, &mut self.app);
    }
}
