//! Hex 视图桥接层：映射 classview 主题色到 egui-hex-view
//!
//! @author sky

use crate::appearance::theme;
use rust_i18n::t;

/// classview 主题色映射到 HexTheme
pub fn hex_theme() -> egui_hex_view::HexTheme {
    egui_hex_view::HexTheme {
        addr_color: theme::TEXT_MUTED,
        addr_hover_color: theme::TEXT_SECONDARY,
        hex_null_color: theme::HEX_NULL,
        hex_printable_color: theme::SYN_NUMBER,
        hex_control_color: theme::TEXT_MUTED,
        hex_high_color: theme::ACCENT_ORANGE,
        ascii_color: theme::SYN_STRING,
        ascii_dot_color: theme::TEXT_MUTED,
        text_primary: theme::TEXT_PRIMARY,
        text_secondary: theme::TEXT_SECONDARY,
        text_muted: theme::TEXT_MUTED,
        accent: theme::VERDIGRIS,
        hover_row_bg: theme::HEX_ROW_HOVER,
        hover_byte_bg: theme::HEX_BYTE_HOVER,
        selection_bg: theme::HEX_SELECTION,
        cursor_bg: theme::HEX_CURSOR,
        separator: theme::HEX_SEPARATOR,
        border: theme::BORDER,
        inspector_bg: theme::BG_DARKEST,
        header_color: theme::TEXT_MUTED,
        header_bg: theme::BG_DARKEST,
        search_bg: theme::verdigris_alpha(25),
        search_current_bg: theme::verdigris_alpha(60),
        labels: egui_hex_view::HexLabels {
            empty: t!("hex.empty").to_string(),
            copy_hex: t!("hex.copy_hex").to_string(),
            copy_ascii: t!("hex.copy_ascii").to_string(),
            copy_offset: t!("hex.copy_offset").to_string(),
            select_all: t!("hex.select_all").to_string(),
            selection: t!("hex.selection").to_string(),
            cursor: t!("hex.cursor").to_string(),
            hover: t!("hex.hover").to_string(),
            bytes: t!("hex.bytes").to_string(),
        },
    }
}
