//! Properties / INI 着色（无 tree-sitter，行级解析）
//!
//! @author sky

use super::{CodeLine, CodeToken};

/// 行级解析 .properties / .ini 文件
pub fn highlight_properties(source: &str) -> Vec<CodeLine> {
    source
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let line_num = (i + 1) as i32;
            let trimmed = line.trim();
            let tokens = if trimmed.is_empty() {
                vec![CodeToken {
                    text: " ".into(),
                    color_id: 0,
                }]
            } else if trimmed.starts_with('#') || trimmed.starts_with('!') {
                // 注释
                vec![CodeToken {
                    text: line.into(),
                    color_id: 5,
                }]
            } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // INI section header
                vec![CodeToken {
                    text: line.into(),
                    color_id: 1,
                }]
            } else if let Some(sep) = line.find('=').or_else(|| line.find(':')) {
                // key = value
                vec![
                    CodeToken {
                        text: line[..sep].into(),
                        color_id: 3,
                    },
                    CodeToken {
                        text: line[sep..sep + 1].into(),
                        color_id: 7,
                    },
                    CodeToken {
                        text: line[sep + 1..].into(),
                        color_id: 2,
                    },
                ]
            } else {
                vec![CodeToken {
                    text: line.into(),
                    color_id: 0,
                }]
            };
            CodeLine { line_num, tokens }
        })
        .collect()
}
