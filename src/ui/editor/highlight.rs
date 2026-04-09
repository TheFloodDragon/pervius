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
mod kotlin;
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
    /// 常量（enum 值、static final 字段）
    Constant,
    /// 方法调用
    MethodCall,
    /// 方法声明
    MethodDecl,
}

impl TokenKind {
    /// 映射到 egui 颜色
    pub fn color(self) -> egui::Color32 {
        match self {
            Self::Plain => theme::SYN_TEXT,
            Self::Keyword => theme::SYN_KEYWORD,
            Self::String => theme::SYN_STRING,
            Self::Type => theme::SYN_TYPE,
            Self::Number => theme::SYN_NUMBER,
            Self::Comment => theme::SYN_COMMENT,
            Self::Annotation => theme::SYN_ANNOTATION,
            Self::Muted => theme::TEXT_MUTED,
            Self::Constant => theme::SYN_CONSTANT,
            Self::MethodCall => theme::SYN_METHOD,
            Self::MethodDecl => theme::SYN_METHOD_DECL,
        }
    }
}

/// 支持高亮的语言（限 JAR 内可能出现的类型）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    /// Kotlin（tree-sitter-kotlin）
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

/// 带 hash 缓存的 layouter 工厂（通用骨架）
///
/// `highlight_fn` 负责将源码转为 LayoutJob，缓存层在此统一处理。
fn cached_layouter(
    mut highlight_fn: impl FnMut(&str) -> LayoutJob,
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
            cached_job = highlight_fn(text);
            cached_hash = hash;
        }
        let mut job = cached_job.clone();
        job.wrap.max_width = wrap_width;
        ui.painter().layout_job(job)
    }
}

/// 供 TextEdit 使用的 layouter 回调工厂（tree-sitter 语言高亮）
pub fn make_layouter(
    lang: Language,
) -> impl FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley> {
    cached_layouter(move |text| highlight_layout(text, lang))
}

/// 字节码高亮的 layouter 回调工厂
pub fn make_bytecode_layouter() -> impl FnMut(&egui::Ui, &str, f32) -> std::sync::Arc<egui::Galley>
{
    cached_layouter(|text| {
        let spans = bytecode::collect_spans(text);
        build_layout_job(text, &spans)
    })
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

fn collect_treesitter_spans(source: &str, lang: Language) -> Vec<Span> {
    let mut parser = tree_sitter::Parser::new();
    let ts_lang: tree_sitter::Language = match lang {
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_sg::LANGUAGE.into(),
        Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
        Language::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Sql => tree_sitter_sequel::LANGUAGE.into(),
        Language::Properties | Language::Plain => return vec![(0, source.len(), TokenKind::Plain)],
    };
    if parser.set_language(&ts_lang).is_err() {
        return vec![(0, source.len(), TokenKind::Plain)];
    }
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![(0, source.len(), TokenKind::Plain)],
    };
    let color_fn: ColorFn = match lang {
        Language::Java => java::classify,
        Language::Kotlin => kotlin::classify,
        Language::Xml => xml::classify,
        Language::Yaml => yaml::classify,
        Language::Json => json::classify,
        Language::Html => html::classify,
        Language::Sql => sql::classify,
        Language::Properties | Language::Plain => return vec![(0, source.len(), TokenKind::Plain)],
    };
    let mut spans = Vec::new();
    collect_node_spans(
        &mut tree.root_node().walk(),
        &mut spans,
        color_fn,
        source.as_bytes(),
    );
    spans.sort_by_key(|&(start, _, _)| start);
    spans
}

/// 各语言着色函数签名：返回 Some 表示命中，None 表示继续深入子节点
type ColorFn = fn(&tree_sitter::Node, &[u8]) -> Option<TokenKind>;

/// 一次深度优先遍历，收集所有叶子 span（字节偏移）
fn collect_node_spans(
    cursor: &mut tree_sitter::TreeCursor,
    spans: &mut Vec<Span>,
    color_fn: ColorFn,
    source: &[u8],
) {
    loop {
        let node = cursor.node();
        let kind = color_fn(&node, source);
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

/// 为搜索预览面板生成逐行 LayoutJob（含语法高亮 + 匹配区间背景标记）
///
/// `match_line` 对应的行会在 `match_ranges`（行内字节偏移）区间叠加 `match_bg` 背景色，
/// 使匹配文本在语法着色之上仍能辨识。
pub fn highlight_per_line(
    lines: &[String],
    bytecode_mode: bool,
    font: egui::FontId,
    match_line: usize,
    match_ranges: &[(usize, usize)],
    match_bg: egui::Color32,
) -> Vec<LayoutJob> {
    let source = lines.join("\n");
    let spans = if bytecode_mode {
        bytecode::collect_spans(&source)
    } else {
        collect_spans(&source, Language::Java)
    };
    // 每行在 source 中的起始字节偏移
    let mut line_starts = Vec::with_capacity(lines.len());
    let mut off = 0usize;
    for line in lines {
        line_starts.push(off);
        off += line.len() + 1;
    }
    let mut jobs = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let ls = line_starts[i];
        let le = ls + line.len();
        // 裁剪 + 转行内相对偏移
        let line_spans: Vec<Span> = spans
            .iter()
            .filter(|&&(s, e, _)| s < le && e > ls)
            .map(|&(s, e, k)| (s.max(ls) - ls, e.min(le) - ls, k))
            .collect();
        let mr = if i == match_line { match_ranges } else { &[] };
        jobs.push(build_line_layout(line, &line_spans, &font, mr, match_bg));
    }
    jobs
}

/// 合并语法 span 与匹配区间，输出单行 LayoutJob
fn build_line_layout(
    line: &str,
    spans: &[Span],
    font: &egui::FontId,
    match_ranges: &[(usize, usize)],
    match_bg: egui::Color32,
) -> LayoutJob {
    let mut breaks = std::collections::BTreeSet::new();
    breaks.insert(0);
    breaks.insert(line.len());
    for &(s, e, _) in spans {
        breaks.insert(s.min(line.len()));
        breaks.insert(e.min(line.len()));
    }
    for &(s, e) in match_ranges {
        breaks.insert(s.min(line.len()));
        breaks.insert(e.min(line.len()));
    }
    let breaks: Vec<usize> = breaks.into_iter().collect();
    let mut job = LayoutJob::default();
    for w in breaks.windows(2) {
        let (start, end) = (w[0], w[1]);
        if start >= end || start >= line.len() {
            continue;
        }
        let end = end.min(line.len());
        let color = spans
            .iter()
            .find(|&&(s, e, _)| s <= start && end <= e)
            .map(|&(_, _, k)| k.color())
            .unwrap_or(TokenKind::Plain.color());
        let in_match = match_ranges.iter().any(|&(s, e)| start >= s && end <= e);
        let mut format = egui::TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        };
        if in_match {
            format.background = match_bg;
        }
        job.append(&line[start..end], 0.0, format);
    }
    job
}
