//! Java 着色规则
//!
//! @author sky

use super::TokenKind;

/// 返回 Some 表示命中着色，None 表示继续深入子节点
pub fn classify(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
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
        "true" | "false" | "null" => Some(TokenKind::Keyword),
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
        // marker_annotation 无参数，整体着色即可
        "marker_annotation" => Some(TokenKind::Annotation),
        // annotation 含参数（可能有字符串），递归进子节点分别着色
        "annotation" => None,
        "@" => Some(TokenKind::Annotation),
        "type_identifier" => Some(TokenKind::Type),
        "identifier" => classify_identifier(node, source),
        _ => None,
    }
}

fn classify_identifier(node: &tree_sitter::Node, source: &[u8]) -> Option<TokenKind> {
    let parent = node.parent()?;
    match parent.kind() {
        // 注解名称 @Foo / @Foo(...)
        "annotation" | "marker_annotation" => return Some(TokenKind::Annotation),
        // 注解限定名称 @java.lang.Override
        "scoped_identifier" => {
            if parent
                .parent()
                .is_some_and(|gp| gp.kind() == "annotation" || gp.kind() == "marker_annotation")
            {
                return Some(TokenKind::Annotation);
            }
        }
        // 类型声明名称
        "class_declaration"
        | "enum_declaration"
        | "interface_declaration"
        | "record_declaration"
        | "annotation_type_declaration" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::Type);
            }
        }
        // enum 常量声明
        "enum_constant" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::Constant);
            }
        }
        // 方法声明 / 构造器
        "method_declaration" | "constructor_declaration" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::MethodDeclaration);
            }
        }
        // 方法调用
        "method_invocation" => {
            if parent
                .child_by_field_name("name")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::MethodCall);
            }
        }
        // 字段访问 object.field
        "field_access" => {
            if parent
                .child_by_field_name("field")
                .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::Constant);
            }
        }
        // 字段声明 private final boolean successful;
        // AST: field_declaration > variable_declarator > identifier(name)
        "variable_declarator" => {
            if parent
                .parent()
                .is_some_and(|gp| gp.kind() == "field_declaration")
                && parent
                    .child_by_field_name("name")
                    .is_some_and(|n| n.id() == node.id())
            {
                return Some(TokenKind::Constant);
            }
        }
        _ => {}
    }
    // ALL_CAPS 启发式：UPPER_SNAKE_CASE 识别为常量（enum 引用、static final 等）
    if let Ok(text) = node.utf8_text(source) {
        if text.len() >= 2 && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
            return Some(TokenKind::Constant);
        }
    }
    None
}
