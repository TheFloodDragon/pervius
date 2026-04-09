//! 编辑器 Tab 数据结构
//!
//! @author sky

use super::highlight::{self, Language};
use super::view_toggle::ActiveView;
use eframe::egui;
use std::sync::Arc;

/// 编辑器 Tab 数据
pub struct EditorTab {
    pub title: String,
    /// 反编译源码（只读）
    pub decompiled: String,
    /// 字节码文本（可编辑）
    pub bytecode: String,
    /// Hex dump 文本（只读）
    pub hex_dump: String,
    pub language: Language,
    pub active_view: ActiveView,
    pub is_modified: bool,
    /// Decompiled 视图的 layouter 缓存
    pub(super) layouter_decompiled: Box<dyn FnMut(&egui::Ui, &str, f32) -> Arc<egui::Galley>>,
}

impl EditorTab {
    pub fn new(
        title: impl Into<String>,
        decompiled: impl Into<String>,
        bytecode: impl Into<String>,
        hex_dump: impl Into<String>,
        language: Language,
    ) -> Self {
        let lang = language;
        Self {
            title: title.into(),
            decompiled: decompiled.into(),
            bytecode: bytecode.into(),
            hex_dump: hex_dump.into(),
            language,
            active_view: ActiveView::Decompiled,
            is_modified: false,
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
        }
    }
}
