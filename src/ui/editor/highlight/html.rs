//! HTML 着色规则
//!
//! @author sky

use super::TokenKind;

pub fn classify(node: &tree_sitter::Node) -> Option<TokenKind> {
    match node.kind() {
        "tag_name" => Some(TokenKind::Keyword),
        "attribute_name" => Some(TokenKind::Type),
        "attribute_value" | "quoted_attribute_value" => Some(TokenKind::String),
        "comment" => Some(TokenKind::Comment),
        "doctype" | "<!DOCTYPE" | "<!doctype" => Some(TokenKind::Annotation),
        "<" | ">" | "</" | "/>" => Some(TokenKind::Muted),
        "text" | "raw_text" => Some(TokenKind::Plain),
        _ => None,
    }
}
