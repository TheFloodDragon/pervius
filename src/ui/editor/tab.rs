//! 编辑器 Tab 数据结构
//!
//! @author sky

use super::highlight::{self, Language};
use super::view_toggle::ActiveView;
use eframe::egui;
use egui_hex_view::HexViewState;
use std::sync::Arc;

/// 编辑器 Tab 数据
pub struct EditorTab {
    pub title: String,
    /// 反编译源码（只读）
    pub decompiled: String,
    /// 字节码文本（可编辑）
    pub bytecode: String,
    /// 原始字节（只读，用于 hex 视图）
    pub raw_bytes: Vec<u8>,
    pub language: Language,
    pub active_view: ActiveView,
    pub is_modified: bool,
    /// Hex 视图交互状态
    pub hex_state: HexViewState,
    /// Decompiled 视图的 layouter 缓存
    pub(super) layouter_decompiled: Box<dyn FnMut(&egui::Ui, &str, f32) -> Arc<egui::Galley>>,
    /// Bytecode 视图的 layouter 缓存
    pub(super) layouter_bytecode: Box<dyn FnMut(&egui::Ui, &str, f32) -> Arc<egui::Galley>>,
}

impl EditorTab {
    pub fn new(
        title: impl Into<String>,
        decompiled: impl Into<String>,
        bytecode: impl Into<String>,
        raw_bytes: Vec<u8>,
        language: Language,
    ) -> Self {
        let lang = language;
        Self {
            title: title.into(),
            decompiled: decompiled.into(),
            bytecode: bytecode.into(),
            raw_bytes,
            language,
            active_view: ActiveView::Decompiled,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
            layouter_bytecode: Box::new(highlight::make_bytecode_layouter()),
        }
    }
}
