//! Kotlin 着色规则（tree-sitter-kotlin）
//!
//! @author sky

use super::TokenKind;

/// 返回 Some 表示命中着色，None 表示继续深入子节点
pub fn classify(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
    match node.kind() {
        // 关键字（叶节点）
        "val" | "var" | "fun" | "class" | "object" | "interface" | "enum" | "typealias" | "if"
        | "else" | "when" | "for" | "do" | "while" | "try" | "catch" | "throw" | "finally"
        | "import" | "package" | "is" | "!is" | "in" | "!in" | "as" | "as?" | "constructor"
        | "init" | "get" | "set" | "return" | "continue" | "break" | "return_at"
        | "continue_at" | "break_at" | "new" => Some(TokenKind::Keyword),
        // 修饰符关键字
        "class_modifier"
        | "member_modifier"
        | "function_modifier"
        | "property_modifier"
        | "platform_modifier"
        | "variance_modifier"
        | "parameter_modifier"
        | "visibility_modifier"
        | "reification_modifier"
        | "inheritance_modifier" => Some(TokenKind::Keyword),
        // this / super
        "this_expression" | "super_expression" => Some(TokenKind::Keyword),
        // 字面量关键字
        "boolean_literal" | "null_literal" => Some(TokenKind::Keyword),
        "integer_literal" | "long_literal" | "hex_literal" | "bin_literal" | "unsigned_literal"
        | "real_literal" => Some(TokenKind::Number),
        "string_literal" | "character_literal" | "character_escape_seq" => Some(TokenKind::String),
        // 注释
        "line_comment" | "multiline_comment" | "shebang_line" => Some(TokenKind::Comment),
        // 注解：递归进子节点，让内部字符串正确着色
        "annotation" => None,
        "@" => Some(TokenKind::Annotation),
        // 类型（注解内的类型标识符着色为注解色）
        "type_identifier" => {
            if ancestors_contain(node, "annotation", 6) {
                Some(TokenKind::Annotation)
            } else {
                Some(TokenKind::Type)
            }
        }
        // 标识符
        "simple_identifier" => classify_identifier(node, source),
        // 标点
        "(" | ")" | "[" | "]" | "{" | "}" | "." | "," | ";" | ":" | "::" => Some(TokenKind::Muted),
        _ => None,
    }
}

fn classify_identifier(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
    let parent = node.parent()?;
    match parent.kind() {
        // 类型声明名称
        "class_declaration" | "object_declaration" => {
            if is_first_identifier_child(&parent, node) {
                return Some(TokenKind::Type);
            }
        }
        // enum 条目
        "enum_entry" => {
            if is_first_identifier_child(&parent, node) {
                return Some(TokenKind::Constant);
            }
        }
        // 函数声明的名称
        "function_declaration" => {
            if is_first_identifier_child(&parent, node) {
                return Some(TokenKind::MethodDecl);
            }
        }
        // 函数调用
        "call_expression" => {
            if is_first_child(&parent, node) {
                return Some(TokenKind::MethodCall);
            }
        }
        // object.func() / object.field 中 navigation_suffix 内的标识符
        "navigation_suffix" => {
            // 向上查找是否处于 call_expression 中（AST 层级可能是 call > nav_expr > nav_suffix）
            let is_call = ancestors_contain(&parent, "call_expression", 3);
            if is_call {
                return Some(TokenKind::MethodCall);
            }
            // 非调用 → 属性/字段访问
            let text = node.utf8_text(source).unwrap_or("");
            if text.len() >= 2 && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                return Some(TokenKind::Constant);
            }
            if text.starts_with(|c: char| c.is_uppercase()) {
                return Some(TokenKind::Type);
            }
            return Some(TokenKind::Constant);
        }
        _ => {}
    }
    let text = node.utf8_text(source).unwrap_or("");
    // ALL_CAPS → 常量
    if text.len() >= 2 && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        return Some(TokenKind::Constant);
    }
    // 大写开头 → 类型
    if text.starts_with(|c: char| c.is_uppercase()) {
        return Some(TokenKind::Type);
    }
    None
}

fn is_first_child(parent: &tree_sitter::Node, node: &tree_sitter::Node) -> bool {
    parent.child(0).is_some_and(|c| c.id() == node.id())
}

fn is_first_identifier_child(parent: &tree_sitter::Node, node: &tree_sitter::Node) -> bool {
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            if child.kind() == "simple_identifier" {
                return child.id() == node.id();
            }
        }
    }
    false
}

/// 在 max_depth 层祖先内查找指定 kind
fn ancestors_contain(node: &tree_sitter::Node, kind: &str, max_depth: usize) -> bool {
    let mut cur = node.parent();
    for _ in 0..max_depth {
        match cur {
            Some(n) if n.kind() == kind => return true,
            Some(n) => cur = n.parent(),
            None => return false,
        }
    }
    false
}
