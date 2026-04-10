//! 编辑器主题配色
//!
//! @author sky

use eframe::egui;
use egui_shell::components::FlatButtonTheme;

/// 语法高亮配色（12 种 token 类型）
#[derive(Clone)]
pub struct SyntaxTheme {
    /// 普通文本
    pub text: egui::Color32,
    /// 关键字
    pub keyword: egui::Color32,
    /// JVM 操作码（字节码指令，区别于语言关键字）
    pub opcode: egui::Color32,
    /// 字符串
    pub string: egui::Color32,
    /// 类型名
    pub type_name: egui::Color32,
    /// 数字
    pub number: egui::Color32,
    /// 注释
    pub comment: egui::Color32,
    /// 注解
    pub annotation: egui::Color32,
    /// 低对比度文本（标点、分隔符）
    pub muted: egui::Color32,
    /// 常量（enum 值、static final）
    pub constant: egui::Color32,
    /// 方法调用
    pub method_call: egui::Color32,
    /// 方法声明
    pub method_declaration: egui::Color32,
}

/// 代码视图配色（gutter + 编辑区 + 搜索匹配）
#[derive(Clone)]
pub struct CodeViewTheme {
    /// 语法高亮
    pub syntax: SyntaxTheme,
    /// 编辑器背景
    pub bg: egui::Color32,
    /// 行号栏背景
    pub gutter_bg: egui::Color32,
    /// 行号文字色
    pub line_number_color: egui::Color32,
    /// 搜索匹配背景（普通匹配）
    pub search_bg: egui::Color32,
    /// 搜索匹配背景（当前匹配）
    pub search_current_bg: egui::Color32,
    /// 代码字体大小
    pub code_font_size: f32,
    /// 行高亮闪烁底色（scroll_to_line 触发，alpha 由动画控制）
    pub line_highlight: egui::Color32,
}

/// 查找栏图标配置
#[derive(Clone)]
pub struct FindBarIcons {
    /// 关闭
    pub close: &'static str,
    /// 下一个
    pub next: &'static str,
    /// 上一个
    pub prev: &'static str,
    /// 正则
    pub regex: &'static str,
    /// 全词匹配
    pub whole_word: &'static str,
    /// 大小写敏感
    pub case_sensitive: &'static str,
    /// 搜索
    pub search: &'static str,
    /// 图标字体族
    pub font: egui::FontFamily,
}

/// 查找栏标签文本
#[derive(Clone)]
pub struct FindBarLabels {
    /// 关闭 tooltip
    pub close: String,
    /// 下一个 tooltip
    pub next: String,
    /// 上一个 tooltip
    pub prev: String,
    /// 正则 tooltip
    pub use_regex: String,
    /// 全词匹配 tooltip
    pub match_word: String,
    /// 大小写 tooltip
    pub match_case: String,
    /// 无结果提示
    pub no_results: String,
    /// 结果计数格式化（current 1-indexed, total）
    pub result_fmt: fn(usize, usize) -> String,
    /// 输入框 hint
    pub hint: String,
}

/// 查找栏主题
#[derive(Clone)]
pub struct FindBarTheme {
    /// island 背景色
    pub bg: egui::Color32,
    /// island 边框色
    pub border: egui::Color32,
    /// 主文字色
    pub text_primary: egui::Color32,
    /// 暗淡文字色
    pub text_muted: egui::Color32,
    /// 无结果文字色
    pub error_color: egui::Color32,
    /// 图标配置
    pub icons: FindBarIcons,
    /// 标签文本
    pub labels: FindBarLabels,
    /// 按钮配色
    pub button: FlatButtonTheme,
}
