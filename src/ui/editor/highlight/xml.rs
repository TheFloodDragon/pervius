//! XML 着色规则
//!
//! @author sky

/// 根据 tree-sitter 节点类型返回 color_id
pub fn node_color_id(node: &tree_sitter::Node) -> i32 {
    match node.kind() {
        // 标签名
        "tag_name" | "end_tag" | "start_tag" | "self_closing_tag" => 1,
        // 属性名
        "attribute_name" => 3,
        // 属性值
        "attribute_value" | "\"" => 2,
        // 文本内容
        "text" | "CharData" => 0,
        // 注释
        "comment" | "Comment" => 5,
        // CDATA
        "cdata_section" | "CDSect" => 2,
        // 处理指令 <?xml ...?>
        "processing_instruction" | "XMLDecl" => 6,
        // 尖括号 / 斜杠等标记
        "<" | ">" | "</" | "/>" | "<?" | "?>" => 7,
        _ => -1,
    }
}
