//! Codicon 图标 codepoint 常量
//!
//! 所有 codepoint 来自 VS Code Codicon 字体，按用途分组。
//! 参考：https://microsoft.github.io/vscode-codicons/dist/codicon.html
//!
//! @author sky

use eframe::egui;

// -- 窗口控制 --

/// 关闭按钮
pub const CHROME_CLOSE: &str = "\u{EAB8}";
/// 最大化按钮
pub const CHROME_MAXIMIZE: &str = "\u{EAB9}";
/// 最小化按钮
pub const CHROME_MINIMIZE: &str = "\u{EABA}";
/// 还原按钮
pub const CHROME_RESTORE: &str = "\u{EABB}";

// -- 文件系统 --

/// 文件图标
pub const FILE: &str = "\u{EA7B}";
/// 关闭的文件夹
pub const FOLDER: &str = "\u{EA83}";
/// 展开的文件夹
pub const FOLDER_OPENED: &str = "\u{EAF7}";
/// 文件树图标（explorer 视图）
pub const FILES: &str = "\u{EAF0}";
/// 符号类（class 图标）
pub const SYMBOL_CLASS: &str = "\u{EB5B}";
/// Java 文件图标（咖啡杯）
pub const JAVA: &str = "\u{EC15}";
/// 符号字段（field 图标）
pub const SYMBOL_FIELD: &str = "\u{EB5F}";
/// 符号方法（method 图标）
pub const SYMBOL_METHOD: &str = "\u{EA8C}";
/// JAR / 归档包图标
pub const PACKAGE: &str = "\u{EB29}";

// -- 箭头 / 折叠 --

/// 向右箭头（折叠状态）
pub const CHEVRON_RIGHT: &str = "\u{EAB6}";
/// 向下箭头（展开状态）
pub const CHEVRON_DOWN: &str = "\u{EAB4}";
/// 向上箭头
pub const CHEVRON_UP: &str = "\u{EAB5}";

// -- 通用 UI --

/// 搜索
pub const SEARCH: &str = "\u{EA6D}";
/// 设置齿轮
pub const SETTINGS_GEAR: &str = "\u{EB51}";
/// 汉堡菜单
pub const MENU: &str = "\u{EB94}";
/// 关闭 / X 号
pub const CLOSE: &str = "\u{EA76}";
/// 更多（三点菜单）
pub const ELLIPSIS: &str = "\u{EA7C}";
/// 结构视图（符号大纲）
pub const SYMBOL_MISC: &str = "\u{EA91}";
/// App logo（beaker 图标）
pub const BEAKER: &str = "\u{EAC4}";
/// 导出
pub const EXPORT: &str = "\u{EBAC}";
/// 复制
pub const COPY: &str = "\u{EBCC}";
/// 修改标记（实心圆点，Unicode 通用符号，非 Codicon 字体）
pub const CIRCLE_FILLED: &str = "\u{25CF}";
/// 图钉（大头针，竖直）
pub const PIN: &str = "\u{EBA0}";
/// 大小写敏感
pub const CASE_SENSITIVE: &str = "\u{EB77}";
/// 正则表达式
pub const REGEX: &str = "\u{EB38}";
/// 全词匹配
pub const WHOLE_WORD: &str = "\u{EB7E}";

/// Codicon 字体族（来自 egui-shell）
pub fn family() -> egui::FontFamily {
    egui_shell::codicon_family()
}
