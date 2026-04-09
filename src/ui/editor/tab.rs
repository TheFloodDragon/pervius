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
    /// JAR 内条目路径（用于 tab 去重 + widget ID）
    pub entry_path: Option<String>,
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
            entry_path: None,
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

    /// 创建 .class 文件 tab（暂无反编译，默认 Hex 视图）
    pub fn new_class(
        title: impl Into<String>,
        entry_path: impl Into<String>,
        raw_bytes: Vec<u8>,
    ) -> Self {
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled: "// Decompiler not yet integrated".into(),
            bytecode: "// Bytecode view not yet available".into(),
            raw_bytes,
            language: Language::Java,
            active_view: ActiveView::Hex,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(Language::Java)),
            layouter_bytecode: Box::new(highlight::make_bytecode_layouter()),
        }
    }

    /// 创建文本文件 tab（默认 Decompiled 视图显示文本内容）
    pub fn new_text(
        title: impl Into<String>,
        entry_path: impl Into<String>,
        text: String,
        raw_bytes: Vec<u8>,
        language: Language,
    ) -> Self {
        let lang = language;
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled: text,
            bytecode: String::new(),
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
