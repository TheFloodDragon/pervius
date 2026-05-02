//! Kotlin 着色规则（tree-sitter-kotlin）
//!
//! @author sky

use super::{Span, TokenKind};

/// 返回 Some 表示命中着色，None 表示继续深入子节点
pub fn classify(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
    let kind = node.kind();
    if is_inside_string_literal(node)
        && (is_literal_dollar_identifier(node, source) || !is_string_interpolation_node(node, kind))
    {
        return Some(TokenKind::String);
    }
    if let Some(token) = classify_jvm_identifier_suffix(node, source) {
        return Some(token);
    }
    match kind {
        // 关键字（叶节点）
        "val" | "var" | "fun" | "class" | "object" | "interface" | "enum" | "typealias" | "if"
        | "else" | "when" | "for" | "do" | "while" | "try" | "catch" | "throw" | "finally"
        | "import" | "package" | "is" | "!is" | "in" | "!in" | "as" | "as?" | "constructor"
        | "init" | "get" | "set" | "return" | "continue" | "break" | "return_at"
        | "continue_at" | "break_at" | "new" | "companion" | "by" | "where" => Some(TokenKind::Keyword),
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
        "string_literal"
        | "line_string_literal"
        | "multi_line_string_literal"
        | "multiline_string_literal"
        | "string_content"
        | "line_str_text"
        | "multi_line_str_text"
        | "line_str_ref"
        | "multi_line_str_ref"
        | "character_literal"
        | "character_escape_seq"
        | "escape_sequence" => Some(TokenKind::String),
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
                Some(TokenKind::Plain)
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
    if ancestors_contain(node, "import_header", 8) {
        return Some(TokenKind::Plain);
    }
    let parent = node.parent()?;
    match parent.kind() {
        // 类型声明名称
        "class_declaration" | "object_declaration" if is_first_identifier_child(&parent, node) => {
            return Some(TokenKind::Plain);
        }
        // enum 条目
        "enum_entry" if is_first_identifier_child(&parent, node) => {
            return Some(TokenKind::Constant);
        }
        // 函数声明的名称
        "function_declaration" if is_first_identifier_child(&parent, node) => {
            return Some(TokenKind::MethodDeclaration);
        }
        // 函数调用
        "call_expression" if is_first_child(&parent, node) => {
            return Some(TokenKind::MethodCall);
        }
        // object.func() / object.field 中 navigation_suffix 内的标识符
        "navigation_suffix" => return classify_navigation_suffix(node, &parent, source),
        _ => {}
    }
    let text = node.utf8_text(source).unwrap_or("");
    if is_keyword_identifier(text) {
        return Some(TokenKind::Keyword);
    }
    if is_upper_snake_case(text) {
        return Some(TokenKind::Constant);
    }
    None
}

fn classify_navigation_suffix(
    node: &tree_sitter::Node,
    parent: &tree_sitter::Node,
    source: &[u8],
) -> Option<TokenKind> {
    if ancestors_contain(parent, "call_expression", 3) {
        return Some(TokenKind::MethodCall);
    }
    let text = node.utf8_text(source).unwrap_or("");
    if is_upper_snake_case(text) {
        return Some(TokenKind::Constant);
    }
    None
}

fn is_upper_snake_case(text: &str) -> bool {
    text.len() >= 2 && text.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

fn is_keyword_identifier(text: &str) -> bool {
    matches!(
        text,
        "abstract"
            | "actual"
            | "annotation"
            | "by"
            | "companion"
            | "const"
            | "crossinline"
            | "data"
            | "delegate"
            | "dynamic"
            | "expect"
            | "external"
            | "field"
            | "file"
            | "final"
            | "infix"
            | "inline"
            | "inner"
            | "internal"
            | "lateinit"
            | "noinline"
            | "open"
            | "operator"
            | "out"
            | "override"
            | "param"
            | "private"
            | "property"
            | "protected"
            | "public"
            | "receiver"
            | "sealed"
            | "setparam"
            | "suspend"
            | "tailrec"
            | "value"
            | "vararg"
            | "where"
    )
}

fn is_literal_dollar_identifier(node: &tree_sitter::Node, source: &[u8]) -> bool {
    (node.kind() == "interpolated_identifier"
        || node.kind() == "interpolation_identifier_start"
        || ancestors_contain(node, "interpolated_identifier", 4))
        && dollar_is_inside_identifier(node.start_byte(), source)
}

fn classify_jvm_identifier_suffix(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
    if !dollar_is_inside_identifier(node.start_byte(), source) {
        return None;
    }
    let text = node.utf8_text(source).unwrap_or("");
    if ancestors_contain(node, "function_declaration", 4) {
        return Some(TokenKind::MethodDeclaration);
    }
    if ancestors_contain(node, "call_expression", 4) || ancestors_contain(node, "navigation_suffix", 4) {
        return Some(TokenKind::MethodCall);
    }
    Some(TokenKind::MethodCall)
}

fn dollar_is_inside_identifier(start: usize, source: &[u8]) -> bool {
    let Some(dollar) = start.checked_sub(1).filter(|&idx| source.get(idx) == Some(&b'$')) else {
        return false;
    };
    dollar > 0 && source.get(dollar - 1).is_some_and(|&b| is_kotlin_identifier_byte(b))
}

fn is_kotlin_identifier_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// 补齐 Kotlin 字符串中 tree-sitter 未覆盖的片段（常见为引号或错误恢复产生的空白区）。
pub(super) fn patch_string_gaps(spans: &mut Vec<Span>, source: &str) {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let end = if source[i..].starts_with("\"\"\"") {
            find_raw_string_end(source, i + 3).unwrap_or(bytes.len())
        } else {
            find_line_string_end(bytes, i + 1)
        };
        fill_string_gaps(spans, source, i, end);
        i = end.max(i + 1);
    }
}

fn fill_string_gaps(spans: &mut Vec<Span>, source: &str, start: usize, end: usize) {
    let mut covered = spans
        .iter()
        .filter_map(|&(s, e, kind)| {
            (kind != TokenKind::Plain && s < end && e > start).then_some((s.max(start), e.min(end)))
        })
        .collect::<Vec<_>>();
    covered.sort_unstable();

    let mut cursor = start;
    for (s, e) in covered {
        if cursor < s {
            push_string_span(spans, source, cursor, s);
        }
        cursor = cursor.max(e);
    }
    push_string_span(spans, source, cursor, end);
}

fn push_string_span(spans: &mut Vec<Span>, source: &str, start: usize, end: usize) {
    if start < end && source.is_char_boundary(start) && source.is_char_boundary(end) {
        spans.push((start, end, TokenKind::String));
    }
}

fn find_raw_string_end(source: &str, from: usize) -> Option<usize> {
    source[from..].find("\"\"\"").map(|offset| from + offset + 3)
}

fn find_line_string_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = (i + 2).min(bytes.len()),
            b'"' => return i + 1,
            b'\n' | b'\r' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn is_inside_string_literal(node: &tree_sitter::Node) -> bool {
    ancestors_contain(node, "string_literal", 16)
        || ancestors_contain(node, "line_string_literal", 16)
        || ancestors_contain(node, "multi_line_string_literal", 16)
        || ancestors_contain(node, "multiline_string_literal", 16)
}

fn is_string_interpolation_node(node: &tree_sitter::Node, kind: &str) -> bool {
    matches!(
        kind,
        "interpolated_expression"
            | "interpolated_identifier"
            | "interpolation_expression_start"
            | "interpolation_expression_end"
            | "interpolation_identifier_start"
            | "line_str_ref"
            | "multi_line_str_ref"
    ) || ancestors_contain(node, "interpolated_expression", 16)
        || ancestors_contain(node, "interpolated_identifier", 16)
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
