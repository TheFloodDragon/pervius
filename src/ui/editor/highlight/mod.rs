//! 语法高亮引擎
//!
//! 通用 tree-sitter 遍历框架 + 各语言着色规则分发。
//! 仅覆盖 JAR 内可能出现的文件类型。
//!
//! @author sky

mod html;
mod java;
mod json;
mod properties;
mod sql;
mod xml;
mod yaml;

use crate::shell::theme;
use eframe::egui;

/// 单个着色 token
#[derive(Clone)]
pub struct CodeToken {
    pub text: String,
    /// 颜色映射 ID：
    /// 0 = text-primary, 1 = keyword, 2 = string, 3 = type, 4 = number,
    /// 5 = comment, 6 = annotation, 7 = muted, 8 = method-call, 9 = method-decl
    pub color_id: i32,
}

/// 一行代码 = 行号 + token 序列
#[derive(Clone)]
pub struct CodeLine {
    pub line_num: i32,
    pub tokens: Vec<CodeToken>,
}

/// 支持高亮的语言（限 JAR 内可能出现的类型）
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

/// color_id → egui 颜色
pub fn token_color(color_id: i32) -> egui::Color32 {
    match color_id {
        1 => theme::SYN_KEYWORD,
        2 => theme::SYN_STRING,
        3 => theme::SYN_TYPE,
        4 => theme::SYN_NUMBER,
        5 => theme::SYN_COMMENT,
        6 => theme::SYN_ANNOTATION,
        7 => theme::TEXT_MUTED,
        8 => theme::SYN_METHOD,
        9 => theme::SYN_METHOD_DECL,
        _ => theme::TEXT_PRIMARY,
    }
}

/// 解析源码，返回着色行序列
pub fn highlight(source: &str, lang: Language) -> Vec<CodeLine> {
    match lang {
        Language::Properties => properties::highlight_properties(source),
        Language::Plain => highlight_plain(source),
        _ => highlight_treesitter(source, lang),
    }
}

/// 旧接口兼容
pub fn highlight_java(source: &str) -> Vec<CodeLine> {
    highlight(source, Language::Java)
}

fn highlight_treesitter(source: &str, lang: Language) -> Vec<CodeLine> {
    let mut parser = tree_sitter::Parser::new();
    let ts_lang: tree_sitter::Language = match lang {
        // Kotlin 复用 Java grammar
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
    let root = tree.root_node();
    let color_fn: fn(&tree_sitter::Node) -> i32 = match lang {
        Language::Java | Language::Kotlin => java::node_color_id,
        Language::Xml => xml::node_color_id,
        Language::Yaml => yaml::node_color_id,
        Language::Json => json::node_color_id,
        Language::Html => html::node_color_id,
        Language::Sql => sql::node_color_id,
        Language::Properties | Language::Plain => unreachable!(),
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut result = Vec::with_capacity(lines.len());
    for (i, line_text) in lines.iter().enumerate() {
        let line_num = (i + 1) as i32;
        let mut tokens = Vec::new();
        if line_text.is_empty() {
            tokens.push(CodeToken {
                text: " ".into(),
                color_id: 0,
            });
        } else {
            let mut cursor = root.walk();
            let mut leaves: Vec<(usize, usize, i32)> = Vec::new();
            collect_leaves(&mut cursor, i, line_text, &mut leaves, color_fn);
            leaves.sort_by_key(|&(start, _, _)| start);
            let line_end = line_text.len();
            let mut pos = 0usize;
            for (start, end, color) in &leaves {
                let s = *start;
                let e = *end;
                if s > pos {
                    tokens.push(CodeToken {
                        text: line_text[pos..s].into(),
                        color_id: 0,
                    });
                }
                if e > pos {
                    let actual_start = if s > pos { s } else { pos };
                    tokens.push(CodeToken {
                        text: line_text[actual_start..e].into(),
                        color_id: *color,
                    });
                    pos = e;
                }
            }
            if pos < line_end {
                tokens.push(CodeToken {
                    text: line_text[pos..line_end].into(),
                    color_id: 0,
                });
            }
        }
        result.push(CodeLine { line_num, tokens });
    }
    result
}

fn highlight_plain(source: &str) -> Vec<CodeLine> {
    source
        .lines()
        .enumerate()
        .map(|(i, line)| CodeLine {
            line_num: (i + 1) as i32,
            tokens: vec![CodeToken {
                text: if line.is_empty() {
                    " ".into()
                } else {
                    line.into()
                },
                color_id: 0,
            }],
        })
        .collect()
}

fn collect_leaves(
    cursor: &mut tree_sitter::TreeCursor,
    target_row: usize,
    line_text: &str,
    result: &mut Vec<(usize, usize, i32)>,
    color_fn: fn(&tree_sitter::Node) -> i32,
) {
    loop {
        let node = cursor.node();
        let start = node.start_position();
        let end = node.end_position();
        if start.row > target_row {
            break;
        }
        if end.row >= target_row && start.row <= target_row {
            let color = color_fn(&node);
            if node.child_count() == 0 || color >= 0 {
                if start.row == target_row || end.row == target_row {
                    let col_start = if start.row == target_row {
                        start.column
                    } else {
                        0
                    };
                    let col_end = if end.row == target_row {
                        end.column
                    } else {
                        line_text.len()
                    };
                    let c = if color >= 0 { color } else { 0 };
                    if col_start < col_end && col_end <= line_text.len() {
                        result.push((col_start, col_end, c));
                    }
                }
                if color >= 0 {
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
            }
            if cursor.goto_first_child() {
                collect_leaves(cursor, target_row, line_text, result, color_fn);
                cursor.goto_parent();
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
