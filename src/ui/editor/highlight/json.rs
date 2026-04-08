//! JSON 着色规则
//!
//! @author sky

pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    match node.kind() {
        "string" | "string_content" | "\"" => {
            // JSON key（pair 的第一个子节点）还是 value
            if let Some(parent) = node.parent() {
                if parent.kind() == "pair" {
                    if parent
                        .child_by_field_name("key")
                        .map_or(false, |n| n.id() == node.id())
                    {
                        return 3;
                    }
                }
            }
            2
        }
        "number" => 4,
        "true" | "false" => 1,
        "null" => 1,
        "comment" => 5,
        _ => -1,
    }
}
