//! Java 着色规则
//!
//! @author sky

use super::TokenKind;

/// 返回 Some 表示命中着色，None 表示继续深入子节点
pub fn classify(node: &tree_sitter::Node) -> Option<TokenKind> {
    match node.kind() {
        "abstract" | "assert" | "break" | "case" | "catch" | "class" | "continue" | "default"
        | "do" | "else" | "enum" | "extends" | "final" | "finally" | "for" | "if"
        | "implements" | "import" | "instanceof" | "interface" | "native" | "new" | "package"
        | "private" | "protected" | "public" | "return" | "static" | "strictfp" | "super"
        | "switch" | "synchronized" | "this" | "throw" | "throws" | "transient" | "try"
        | "void" | "volatile" | "while" | "var" | "record" | "sealed" | "permits" | "yield"
        | "boolean" | "byte" | "char" | "double" | "float" | "int" | "long" | "short" => {
            Some(TokenKind::Keyword)
        }
        "true" | "false" | "null" => Some(TokenKind::Number),
        "string_literal" | "character_literal" | "text_block" | "string_fragment" | "\"" => {
            Some(TokenKind::String)
        }
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal" => Some(TokenKind::Number),
        "line_comment" | "block_comment" => Some(TokenKind::Comment),
        "marker_annotation" | "annotation" | "@" => Some(TokenKind::Annotation),
        "type_identifier" => Some(TokenKind::Type),
        "identifier" => classify_identifier(node),
        _ => None,
    }
}

fn classify_identifier(node: &tree_sitter::Node) -> Option<TokenKind> {
    let parent = node.parent()?;
    match parent.kind() {
        "method_declaration" | "constructor_declaration" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::MethodDecl);
            }
        }
        "method_invocation" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::MethodCall);
            }
        }
        _ => {}
    }
    None
}
