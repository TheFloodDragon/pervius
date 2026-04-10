//! XML 着色规则
//!
//! @author sky

use super::TokenKind;

pub fn classify(node: &tree_sitter::Node, _source: &[u8]) -> Option<TokenKind> {
    match node.kind() {
        "tag_name" | "end_tag" | "start_tag" | "self_closing_tag" => Some(TokenKind::Keyword),
        "attribute_name" => Some(TokenKind::Type),
        "attribute_value" | "\"" => Some(TokenKind::String),
        "text" | "CharData" => Some(TokenKind::Plain),
        "comment" | "Comment" => Some(TokenKind::Comment),
        "cdata_section" | "CDSect" => Some(TokenKind::String),
        "processing_instruction" | "XMLDecl" => Some(TokenKind::Annotation),
        "<" | ">" | "</" | "/>" | "<?" | "?>" => Some(TokenKind::Muted),
        _ => None,
    }
}
