//! 语法高亮引擎
//!
//! 通用 tree-sitter 遍历框架 + 各语言着色规则分发。
//! 输出 egui LayoutJob 供 TextEdit 直接使用。
//! 仅覆盖 JAR 内可能出现的文件类型。
//!
//! @author sky

mod bytecode;
mod html;
mod java;
mod json;
mod properties;
mod sql;
mod xml;
mod yaml;

use crate::shell::theme;
use eframe::egui;
use eframe::egui::text::LayoutJob;

/// 语法 token 类型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    /// 普通文本
    Plain,
    Keyword,
    String,
    Type,
    Number,
    Comment,
    Annotation,
    /// 低对比度文本（标点、分隔符）
    Muted,
    /// 方法调用
    MethodCall,
    /// 方法声明
    MethodDecl,
}

impl TokenKind {
    /// 映射到 egui 颜色
    pub fn color(self) -> egui::Color32 {
        match self {
            Self::Plain => theme::TEXT_PRIMARY,
            Self::Keyword => theme::SYN_KEYWORD,
            Self::String => theme::SYN_STRING,
            Self::Type => theme::SYN_TYPE,
            Self::Number => theme::SYN_NUMBER,
            Self::Comment => theme::SYN_COMMENT,
            Self::Annotation => theme::SYN_ANNOTATION,
            Self::Muted => theme::TEXT_MUTED,
            Self::MethodCall => theme::SYN_METHOD,
            Self::MethodDecl => theme::SYN_METHOD_DECL,
        }
    }
}

/// 支持高亮的语言（限 JAR 内可能出现的类型）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    /// Kotlin 复用 Java grammar（关键字高度重叠，tree-sitter-kotlin 版本不兼容）
    Kotlin,
    Xml,
    Yaml,
    Json,
    Html,
    Sql,
    Properties,
    Plain,
}

impl Language {
    /// 从文件扩展名推断
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            "xml" | "fxml" | "pom" => Self::Xml,
            "yml" | "yaml" => Self::Yaml,
            "json" | "jsonc" | "mcmeta" => Self::Json,
            "html" | "htm" => Self::Html,
            "sql" => Self::Sql,
            "properties" | "cfg" | "ini" | "toml" => Self::Properties,
            _ => Self::Plain,
        }
    }

    /// 从文件名推断
    pub fn from_filename(name: &str) -> Self {
        if let Some(ext) = name.rsplit('.').next() {
            let lang = Self::from_extension(ext);
            if lang != Self::Plain {
                return lang;
            }
        }
        let upper = name.to_uppercase();
        if upper == "MANIFEST.MF" || upper.ends_with(".MF") {
            return Self::Properties;
        }
        Self::Plain
    }
}

/// 代码字体
const CODE_FONT: egui::FontId = egui::FontId::monospace(13.0);

/// 解析源码为 LayoutJob（供 TextEdit layouter 使用）
pub fn highlight_layout(source: &str, lang: Language) -> LayoutJob {
    let spans = collect_spans(source, lang);
    build_layout_job(source, &spans)
}

/// 供 TextEdit 使用的 layouter 回调工厂
///
/// 返回一个闭包，TextEdit 每次需要重新排版时调用。
/// 内部用上次的 hash 做缓存，源码不变时返回上次结果。
pub fn make_layouter(
    lang: Language,
) -> impl FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley> {
    let mut cached_hash: u64 = 0;
    let mut cached_job = LayoutJob::default();
    move |ui: &egui::Ui, text: &str, wrap_width: f32| {
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if hash != cached_hash {
            cached_job = highlight_layout(text, lang);
            cached_hash = hash;
        }
        let mut job = cached_job.clone();
        job.wrap.max_width = wrap_width;
        ui.painter().layout_job(job)
    }
}

/// 一个 span = (byte_start, byte_end, kind)
type Span = (usize, usize, TokenKind);

/// 收集所有着色 span（字节偏移）
fn collect_spans(source: &str, lang: Language) -> Vec<Span> {
    match lang {
        Language::Properties => properties::collect_spans(source),
        Language::Plain => vec![(0, source.len(), TokenKind::Plain)],
        _ => collect_treesitter_spans(source, lang),
    }
}

/// 字节码高亮的 layouter 回调工厂
pub fn make_bytecode_layouter() -> impl FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley>
{
    let mut cached_hash: u64 = 0;
    let mut cached_job = LayoutJob::default();
    move |ui: &egui::Ui, text: &str, wrap_width: f32| {
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if hash != cached_hash {
            let spans = bytecode::collect_spans(text);
            cached_job = build_layout_job(text, &spans);
            cached_hash = hash;
        }
        let mut job = cached_job.clone();
        job.wrap.max_width = wrap_width;
        ui.painter().layout_job(job)
    }
}

fn collect_treesitter_spans(source: &str, lang: Language) -> Vec<Span> {
    let mut parser = tree_sitter::Parser::new();
    let ts_lang: tree_sitter::Language = match lang {
        Language::Java | Language::Kotlin => tree_sitter_java::LANGUAGE.into(),
        Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
        Language::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Sql => tree_sitter_sequel::LANGUAGE.into(),
        Language::Properties | Language::Plain => unreachable!(),
    };
    parser
        .set_language(&ts_lang)
        .expect("Failed to load grammar");
    let tree = parser.parse(source, None).expect("Failed to parse");
    let color_fn: ColorFn = match lang {
        Language::Java | Language::Kotlin => java::classify,
        Language::Xml => xml::classify,
        Language::Yaml => yaml::classify,
        Language::Json => json::classify,
        Language::Html => html::classify,
        Language::Sql => sql::classify,
        Language::Properties | Language::Plain => unreachable!(),
    };
    let mut spans = Vec::new();
    collect_node_spans(&mut tree.root_node().walk(), &mut spans, color_fn);
    spans.sort_by_key(|&(start, _, _)| start);
    spans
}

/// 各语言着色函数签名：返回 Some 表示命中，None 表示继续深入子节点
type ColorFn = fn(&tree_sitter::Node) -> Option<TokenKind>;

/// 一次深度优先遍历，收集所有叶子 span（字节偏移）
fn collect_node_spans(
    cursor: &mut tree_sitter::TreeCursor,
    spans: &mut Vec<Span>,
    color_fn: ColorFn,
) {
    loop {
        let node = cursor.node();
        let kind = color_fn(&node);
        if kind.is_some() || node.child_count() == 0 {
            spans.push((
                node.start_byte(),
                node.end_byte(),
                kind.unwrap_or(TokenKind::Plain),
            ));
            if !advance_cursor(cursor) {
                return;
            }
            continue;
        }
        if cursor.goto_first_child() {
            continue;
        }
        if !advance_cursor(cursor) {
            return;
        }
    }
}

/// 移动到下一个兄弟，若无兄弟则回溯父节点。返回 false 表示遍历结束
fn advance_cursor(cursor: &mut tree_sitter::TreeCursor) -> bool {
    if cursor.goto_next_sibling() {
        return true;
    }
    loop {
        if !cursor.goto_parent() {
            return false;
        }
        if cursor.goto_next_sibling() {
            return true;
        }
    }
}

/// 从排序好的 span 列表构建 LayoutJob，自动填充 gap
fn build_layout_job(source: &str, spans: &[Span]) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut pos = 0usize;
    for &(start, end, kind) in spans {
        // gap 填充（未着色区域用 Plain）
        if start > pos {
            append_section(&mut job, &source[pos..start], TokenKind::Plain);
        }
        let actual_start = start.max(pos);
        if end > actual_start {
            append_section(&mut job, &source[actual_start..end], kind);
            pos = end;
        }
    }
    // 尾部剩余
    if pos < source.len() {
        append_section(&mut job, &source[pos..], TokenKind::Plain);
    }
    job
}

fn append_section(job: &mut LayoutJob, text: &str, kind: TokenKind) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: CODE_FONT,
            color: kind.color(),
            ..Default::default()
        },
    );
}
