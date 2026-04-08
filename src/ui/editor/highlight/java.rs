//! Java 着色规则
//!
//! @author sky

/// 根据 tree-sitter 节点类型返回 color_id，-1 表示需继续向下遍历
pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    match node.kind() {
        "abstract" | "assert" | "break" | "case" | "catch" | "class" | "continue" | "default"
        | "do" | "else" | "enum" | "extends" | "final" | "finally" | "for" | "if"
        | "implements" | "import" | "instanceof" | "interface" | "native" | "new" | "package"
        | "private" | "protected" | "public" | "return" | "static" | "strictfp" | "super"
        | "switch" | "synchronized" | "this" | "throw" | "throws" | "transient" | "try"
        | "void" | "volatile" | "while" | "var" | "record" | "sealed" | "permits" | "yield"
        | "boolean" | "byte" | "char" | "double" | "float" | "int" | "long" | "short" => 1,
        "true" | "false" | "null" => 4,
        "string_literal" | "character_literal" | "text_block" | "string_fragment" | "\"" => 2,
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal" => 4,
        "line_comment" | "block_comment" => 5,
        "marker_annotation" | "annotation" | "@" => 6,
        "type_identifier" => 3,
        "identifier" => identifier_color(node),
        _ => -1,
    }
}

fn identifier_color(node: &tree_sitter::Node) -> i32 {
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "method_declaration" | "constructor_declaration" => {
                if parent
                    .child_by_field_name("name")
                    .map_or(false, |n| n.id() == node.id())
                {
                    return 9;
                }
            }
            "method_invocation" => {
                if parent
                    .child_by_field_name("name")
                    .map_or(false, |n| n.id() == node.id())
                {
                    return 8;
                }
            }
            _ => {}
        }
    }
    -1
}
