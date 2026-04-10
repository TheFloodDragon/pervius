//! 跨模块复用的通用 widget
//!
//! @author sky

use crate::appearance::theme;
pub use egui_shell::components::{FlatButton, FlatButtonTheme};

/// app 默认按钮配色
pub fn flat_button_theme() -> FlatButtonTheme {
    FlatButtonTheme {
        text_primary: theme::TEXT_PRIMARY,
        text_active: theme::VERDIGRIS,
        text_inactive: theme::TEXT_SECONDARY,
        bg_hover: theme::BG_HOVER,
        bg_pressed: theme::BG_LIGHT,
    }
}
