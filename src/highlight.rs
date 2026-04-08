//! Java 语法高亮
//!
//! 基于 tree-sitter 解析 Java 源码，输出 Slint CodeLine 模型。
//!
//! @author sky

use std::rc::Rc;

slint::include_modules!();

/// color-id 映射：
/// 0 = text-primary（默认）
/// 1 = keyword
/// 2 = string
/// 3 = type
/// 4 = number
/// 5 = comment
/// 6 = annotation
/// 7 = muted
/// 8 = method-call
/// 9 = method-decl
fn node_color_id(node: &tree_sitter::Node) -> i32 {
    let kind = node.kind();
    match kind {
        // 关键字
        "abstract" | "assert" | "break" | "case" | "catch" | "class" | "continue" | "default"
        | "do" | "else" | "enum" | "extends" | "final" | "finally" | "for" | "if"
        | "implements" | "import" | "instanceof" | "interface" | "native" | "new" | "package"
        | "private" | "protected" | "public" | "return" | "static" | "strictfp" | "super"
        | "switch" | "synchronized" | "this" | "throw" | "throws" | "transient" | "try"
        | "void" | "volatile" | "while" | "var" | "record" | "sealed" | "permits" | "yield"
        | "boolean" | "byte" | "char" | "double" | "float" | "int" | "long" | "short" => 1,
        "true" | "false" | "null" => 4,
        // 字符串
        "string_literal" | "character_literal" | "text_block" | "string_fragment" | "\"" => 2,
        // 数字
        "decimal_integer_literal"
        | "hex_integer_literal"
        | "octal_integer_literal"
        | "binary_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal" => 4,
        // 注释
        "line_comment" | "block_comment" => 5,
        // 注解
        "marker_annotation" | "annotation" => 6,
        "@" => 6,
        // 类型名
        "type_identifier" => 3,
        // 标识符：根据父节点判断是方法声明、方法调用还是普通标识符
        "identifier" => {
            if let Some(parent) = node.parent() {
                match parent.kind() {
                    "method_declaration" | "constructor_declaration" => {
                        // 检查是否是方法名（通常是 name 字段）
                        if parent
                            .child_by_field_name("name")
                            .map_or(false, |n| n.id() == node.id())
                        {
                            return 9;
                        }
                        -1
                    }
                    "method_invocation" => {
                        if parent
                            .child_by_field_name("name")
                            .map_or(false, |n| n.id() == node.id())
                        {
                            return 8;
                        }
                        -1
                    }
                    _ => -1,
                }
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn highlight_java(source: &str) -> Rc<slint::VecModel<CodeLine>> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("Failed to load Java grammar");
    let tree = parser.parse(source, None).expect("Failed to parse");
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();
    let model = Rc::new(slint::VecModel::<CodeLine>::default());
    for (i, line_text) in lines.iter().enumerate() {
        let line_num = (i + 1) as i32;
        let tokens_model = Rc::new(slint::VecModel::<CodeToken>::default());
        if line_text.is_empty() {
            tokens_model.push(CodeToken {
                text: " ".into(),
                color_id: 0,
            });
        } else {
            // 收集该行所有叶子节点
            let mut cursor = root.walk();
            let mut leaves: Vec<(usize, usize, i32)> = Vec::new();
            collect_leaves(&mut cursor, i, line_text, &mut leaves);
            // 按列排序
            leaves.sort_by_key(|&(start, _, _)| start);
            // 合并覆盖，填充空隙
            let line_end = line_text.len();
            let mut pos = 0usize;
            for (start, end, color) in &leaves {
                let s = *start;
                let e = *end;
                if s > pos {
                    // 未覆盖的部分用默认色
                    tokens_model.push(CodeToken {
                        text: line_text[pos..s].into(),
                        color_id: 0,
                    });
                }
                if e > pos {
                    let actual_start = if s > pos { s } else { pos };
                    tokens_model.push(CodeToken {
                        text: line_text[actual_start..e].into(),
                        color_id: *color,
                    });
                    pos = e;
                }
            }
            if pos < line_end {
                tokens_model.push(CodeToken {
                    text: line_text[pos..line_end].into(),
                    color_id: 0,
                });
            }
        }
        model.push(CodeLine {
            line_num,
            tokens: slint::ModelRc::from(tokens_model),
        });
    }
    model
}

fn collect_leaves(
    cursor: &mut tree_sitter::TreeCursor,
    target_row: usize,
    line_text: &str,
    result: &mut Vec<(usize, usize, i32)>,
) {
    loop {
        let node = cursor.node();
        let start = node.start_position();
        let end = node.end_position();
        // 跳过不在目标行的节点
        if start.row > target_row {
            break;
        }
        if end.row >= target_row && start.row <= target_row {
            let color = node_color_id(&node);
            if node.child_count() == 0 || color >= 0 {
                // 叶节点或已确定颜色的节点
                if start.row == target_row || end.row == target_row {
                    let col_start = if start.row == target_row {
                        start.column
                    } else {
                        0
                    };
                    let col_end = if end.row == target_row {
                        end.column
                    } else {
                        line_text.len()
                    };
                    let c = if color >= 0 { color } else { 0 };
                    if col_start < col_end && col_end <= line_text.len() {
                        result.push((col_start, col_end, c));
                    }
                }
                // 如果确定了颜色就不继续深入
                if color >= 0 {
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                    continue;
                }
            }
            // 深入子节点
            if cursor.goto_first_child() {
                collect_leaves(cursor, target_row, line_text, result);
                cursor.goto_parent();
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
