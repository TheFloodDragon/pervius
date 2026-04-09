//! 文件树模块
//!
//! @author sky

mod build;
mod node;
mod render;

pub use build::build_tree;
pub use node::TreeNode;
pub use render::{first_match, render_tree};

/// 展开到指定路径的节点（沿途展开所有祖先），返回是否找到
pub fn reveal(nodes: &mut [TreeNode], target: &str) -> bool {
    for node in nodes {
        if !node.is_folder && node.path == target {
            return true;
        }
        if node.has_children() && reveal(&mut node.children, target) {
            node.expanded = true;
            return true;
        }
    }
    false
}
