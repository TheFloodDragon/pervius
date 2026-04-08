//! YAML 着色规则
//!
//! @author sky

/// 根据 tree-sitter 节点类型返回 color_id
pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    match node.kind() {
        // key
        "block_mapping_pair" => -1,
        "flow_node" => -1,
        // 字符串值
        "double_quote_scalar" | "single_quote_scalar" | "string_scalar" => 2,
        // 数字
        "integer_scalar" | "float_scalar" => 4,
        // 布尔 / null
        "boolean_scalar" | "null_scalar" => 1,
        // 注释
        "comment" => 5,
        // tag / anchor
        "tag" | "anchor" | "alias" => 6,
        // key 名（block_mapping_pair 的第一个子节点是 key）
        "block_scalar" => 2,
        _ => {
            // YAML key 判断：如果父节点是 block_mapping_pair 且自己是 key 字段
            if let Some(parent) = node.parent() {
                if parent.kind() == "block_mapping_pair" || parent.kind() == "flow_pair" {
                    if parent
                        .child_by_field_name("key")
                        .map_or(false, |n| n.id() == node.id())
                    {
                        return 3;
                    }
                }
            }
            -1
        }
    }
}
