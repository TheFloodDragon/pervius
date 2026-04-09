//! Properties / INI 着色（无 tree-sitter，行级解析）
//!
//! @author sky

use super::{Span, TokenKind};

/// 行级解析 .properties / .ini 文件，返回字节偏移 span
pub fn collect_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for line in source.split('\n') {
        // 保留换行符的完整行长度
        let line_len = line.len();
        let trimmed = line.trim_end_matches('\r').trim();
        if trimmed.is_empty() {
            // 空行，Plain
            spans.push((offset, offset + line_len, TokenKind::Plain));
        } else if trimmed.starts_with('#') || trimmed.starts_with('!') {
            // 注释
            spans.push((offset, offset + line_len, TokenKind::Comment));
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // INI section header
            spans.push((offset, offset + line_len, TokenKind::Keyword));
        } else if let Some(sep_rel) = line.find('=').or_else(|| line.find(':')) {
            // key = value
            spans.push((offset, offset + sep_rel, TokenKind::Type));
            spans.push((offset + sep_rel, offset + sep_rel + 1, TokenKind::Muted));
            if sep_rel + 1 < line_len {
                spans.push((offset + sep_rel + 1, offset + line_len, TokenKind::String));
            }
        } else {
            spans.push((offset, offset + line_len, TokenKind::Plain));
        }
        // +1 for '\n'
        offset += line_len + 1;
    }
    spans
}
