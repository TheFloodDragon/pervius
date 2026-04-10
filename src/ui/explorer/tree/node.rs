//! 文件树节点数据
//!
//! @author sky

use crate::appearance::{codicon, theme};
use eframe::egui;

/// 文件树节点
pub struct TreeNode {
    /// 显示名称
    pub label: String,
    /// 唯一路径标识（文件: 条目路径, 目录: 以 / 结尾, 根: 空字符串）
    pub path: String,
    pub is_folder: bool,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// 是否为 class 文件（含内部类）
    pub fn is_class(&self) -> bool {
        self.label.ends_with(".class") || self.label.starts_with('$')
    }

    pub fn icon(&self) -> &'static str {
        if self.path.is_empty() && self.is_folder {
            codicon::PACKAGE
        } else if self.is_folder {
            if self.expanded {
                codicon::FOLDER_OPENED
            } else {
                codicon::FOLDER
            }
        } else if self.is_class() {
            codicon::JAVA
        } else {
            codicon::FILE
        }
    }

    pub fn icon_color(&self) -> egui::Color32 {
        if self.is_folder {
            theme::ACCENT_ORANGE
        } else if self.is_class() {
            if self.label.starts_with('$') {
                theme::TEXT_SECONDARY
            } else {
                theme::VERDIGRIS
            }
        } else {
            theme::TEXT_SECONDARY
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}
