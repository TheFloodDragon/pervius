//! 主题色常量
//!
//! 所有 hex 值标注在注释中。
//!
//! @author sky

use eframe::egui;
use rust_i18n::t;

use super::codicon;

/// 最深背景 #111112（Island 底色）
pub const BG_DARKEST: egui::Color32 = egui::Color32::from_rgb(17, 17, 18);
/// 行号栏背景 #131314（介于 island 底色与面板底色之间）
pub const BG_GUTTER: egui::Color32 = egui::Color32::from_rgb(19, 19, 20);
/// 主背景 #151516（窗口底色、Header、StatusBar）
pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(21, 21, 22);
/// 中层背景 #1C1C1E（输入框、编辑区、ViewToggle 容器）
pub const BG_MEDIUM: egui::Color32 = egui::Color32::from_rgb(28, 28, 30);
/// 浅层背景 #252527（关闭按钮 hover 等）
pub const BG_LIGHT: egui::Color32 = egui::Color32::from_rgb(37, 37, 39);
/// 悬停背景 #2E2E31
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(46, 46, 49);

/// 主边框 #2E2E30
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(46, 46, 48);
/// 浅边框 #3A3A3D
pub const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgb(58, 58, 61);

/// 主要文字 #ECECEF
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(236, 236, 239);
/// 次要文字 #A0A0AB
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(160, 160, 171);
/// 暗淡文字 #5C5C6A
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(92, 92, 106);

/// 主色调铜绿 #43B3AE
pub const VERDIGRIS: egui::Color32 = egui::Color32::from_rgb(67, 179, 174);
/// 辅助绿 #6EE7B7
pub const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(110, 231, 183);
/// 辅助橙 #E0A458
pub const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(224, 164, 88);
/// 辅助红 #E06C75
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(224, 108, 117);
/// 辅助青 #7EC8C8
pub const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(126, 200, 200);

/// 代码默认文字 #BCBEC4
pub const SYN_TEXT: egui::Color32 = egui::Color32::from_rgb(188, 190, 196);
/// 关键字 #CF8E6D
pub const SYN_KEYWORD: egui::Color32 = egui::Color32::from_rgb(207, 142, 109);
/// 字符串 #6AAB73
pub const SYN_STRING: egui::Color32 = egui::Color32::from_rgb(106, 171, 115);
/// 类型（与默认文字同色，IntelliJ 不对类引用做特殊着色）
pub const SYN_TYPE: egui::Color32 = SYN_TEXT;
/// 常量 / 字段 #C77DBB
pub const SYN_CONSTANT: egui::Color32 = egui::Color32::from_rgb(199, 125, 187);
/// 数字 #2AACB8
pub const SYN_NUMBER: egui::Color32 = egui::Color32::from_rgb(42, 172, 184);
/// 注释 #7A7E85
pub const SYN_COMMENT: egui::Color32 = egui::Color32::from_rgb(122, 126, 133);
/// 注解 #B3AE60
pub const SYN_ANNOTATION: egui::Color32 = egui::Color32::from_rgb(179, 174, 96);
/// 方法调用（与默认文字同色）
pub const SYN_METHOD: egui::Color32 = SYN_TEXT;
/// 方法声明 #56A8F5
pub const SYN_METHOD_DECL: egui::Color32 = egui::Color32::from_rgb(86, 168, 245);

/// 标题栏按钮 hover #2A2A2F
pub const CAPTION_HOVER: egui::Color32 = egui::Color32::from_rgb(42, 42, 47);
/// 关闭按钮 hover #C42B1C（Win11 风格红）
pub const CLOSE_HOVER: egui::Color32 = egui::Color32::from_rgb(196, 43, 28);

/// 标题栏高度
pub const TITLE_BAR_HEIGHT: f32 = 36.0;
/// 文件面板宽度
pub const FILE_PANEL_WIDTH: f32 = 260.0;
/// 状态栏高度
pub const STATUS_BAR_HEIGHT: f32 = 24.0;
/// Island 圆角半径
pub const ISLAND_RADIUS: u8 = 8;
/// Island 之间的间距
pub const ISLAND_GAP: f32 = 6.0;
/// Island 到窗口左右边缘的水平边距
pub const ISLAND_MARGIN_H: f32 = 6.0;
/// Island 到窗口上下边缘的垂直边距
pub const ISLAND_MARGIN_V: f32 = 4.0;

/// 铜绿色带透明度（用于选中高亮等）
pub fn verdigris_alpha(alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(
        (67u16 * alpha as u16 / 255) as u8,
        (179u16 * alpha as u16 / 255) as u8,
        (174u16 * alpha as u16 / 255) as u8,
        alpha,
    )
}

/// 浮动窗口（Search / Settings 等）的 WindowConfig
pub fn window_config() -> egui_shell::WindowConfig {
    egui_shell::WindowConfig {
        frame: egui::Frame {
            fill: BG_GUTTER,
            corner_radius: egui::CornerRadius::same(8),
            stroke: egui::Stroke::new(1.0, BORDER),
            inner_margin: egui::Margin::same(0),
            shadow: egui::Shadow {
                spread: 2,
                blur: 20,
                offset: [0, 4],
                color: egui::Color32::from_black_alpha(80),
            },
            ..Default::default()
        },
        header_height: 32.0,
        pin_icon: codicon::PIN,
        pin_tooltip: t!("window.pin").to_string(),
        unpin_tooltip: t!("window.unpin").to_string(),
    }
}

/// 设置面板 widget 主题
pub fn settings_theme() -> egui_shell::components::SettingsTheme {
    egui_shell::components::SettingsTheme {
        text_primary: TEXT_PRIMARY,
        text_secondary: TEXT_SECONDARY,
        text_muted: TEXT_MUTED,
        bg_hover: BG_HOVER,
        bg_light: BG_LIGHT,
        bg_medium: BG_MEDIUM,
        bg_sidebar: BG_DARKEST,
        border: BORDER,
        accent: VERDIGRIS,
        icon_font: egui::FontFamily::Name("codicon".into()),
        chevron_icon: codicon::CHEVRON_DOWN,
    }
}

/// 菜单项主题
pub fn menu_theme() -> egui_shell::components::MenuTheme {
    egui_shell::components::MenuTheme {
        text_primary: TEXT_PRIMARY,
        text_muted: TEXT_MUTED,
        bg_hover: BG_HOVER,
    }
}

/// 状态栏主题
pub fn status_bar_theme() -> egui_shell::components::StatusBarTheme {
    egui_shell::components::StatusBarTheme {
        bg: BG_DARK,
        separator: BORDER,
    }
}

/// 确认弹窗主题
pub fn confirm_theme() -> egui_shell::components::ConfirmTheme {
    egui_shell::components::ConfirmTheme {
        frame: egui::Frame {
            fill: BG_LIGHT,
            stroke: egui::Stroke::new(1.0, BORDER_LIGHT),
            corner_radius: egui::CornerRadius::same(8),
            shadow: egui::Shadow {
                spread: 2,
                blur: 24,
                offset: [0, 8],
                color: egui::Color32::from_black_alpha(100),
            },
            ..Default::default()
        },
        title_color: TEXT_PRIMARY,
        message_color: TEXT_SECONDARY,
        separator: BORDER,
        backdrop: egui::Color32::from_black_alpha(80),
        button: egui_shell::components::FlatButtonTheme {
            text_primary: TEXT_PRIMARY,
            text_active: VERDIGRIS,
            text_inactive: TEXT_SECONDARY,
            bg_hover: BG_HOVER,
            bg_pressed: BG_LIGHT,
        },
    }
}

/// 编辑器代码视图主题
pub fn editor_theme() -> egui_editor::CodeViewTheme {
    egui_editor::CodeViewTheme {
        syntax: egui_editor::SyntaxTheme {
            text: SYN_TEXT,
            keyword: SYN_KEYWORD,
            string: SYN_STRING,
            type_name: SYN_TYPE,
            number: SYN_NUMBER,
            comment: SYN_COMMENT,
            annotation: SYN_ANNOTATION,
            muted: TEXT_MUTED,
            constant: SYN_CONSTANT,
            method_call: SYN_METHOD,
            method_declaration: SYN_METHOD_DECL,
        },
        bg: BG_DARKEST,
        gutter_bg: BG_GUTTER,
        line_number_color: TEXT_MUTED,
        search_bg: verdigris_alpha(40),
        search_current_bg: verdigris_alpha(100),
        code_font_size: 13.0,
    }
}

/// 编辑器查找栏主题
pub fn find_bar_theme() -> egui_editor::FindBarTheme {
    egui_editor::FindBarTheme {
        bg: BG_DARK,
        border: BORDER,
        text_primary: TEXT_PRIMARY,
        text_muted: TEXT_MUTED,
        error_color: ACCENT_RED,
        icons: egui_editor::theme::FindBarIcons {
            close: codicon::CLOSE,
            next: codicon::CHEVRON_DOWN,
            prev: codicon::CHEVRON_UP,
            regex: codicon::REGEX,
            whole_word: codicon::WHOLE_WORD,
            case_sensitive: codicon::CASE_SENSITIVE,
            search: codicon::SEARCH,
            font: codicon::family(),
        },
        labels: egui_editor::theme::FindBarLabels {
            close: t!("find.close").to_string(),
            next: t!("find.next").to_string(),
            prev: t!("find.prev").to_string(),
            use_regex: t!("find.use_regex").to_string(),
            match_word: t!("find.match_word").to_string(),
            match_case: t!("find.match_case").to_string(),
            no_results: t!("find.no_results").to_string(),
            result_fmt: |current, total| {
                t!("find.result_count", current = current, total = total).to_string()
            },
            hint: t!("find.hint").to_string(),
        },
        button: egui_shell::components::FlatButtonTheme {
            text_primary: TEXT_PRIMARY,
            text_active: VERDIGRIS,
            text_inactive: TEXT_MUTED,
            bg_hover: BG_HOVER,
            bg_pressed: BG_LIGHT,
        },
    }
}
