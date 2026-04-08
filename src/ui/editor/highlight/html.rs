//! HTML 着色规则
//!
//! @author sky

pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    match node.kind() {
        "tag_name" => 1,
        "attribute_name" => 3,
        "attribute_value" | "quoted_attribute_value" => 2,
        "comment" => 5,
        "doctype" | "<!DOCTYPE" | "<!doctype" => 6,
        "<" | ">" | "</" | "/>" => 7,
        "text" | "raw_text" => 0,
        _ => -1,
    }
}
