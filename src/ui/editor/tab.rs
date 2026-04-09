//! 编辑器 Tab 数据结构
//!
//! @author sky

use super::highlight::{self, Language};
use super::view_toggle::ActiveView;
use eframe::egui;
use egui_hex_view::HexViewState;
use rust_i18n::t;
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
    /// .class 文件才显示三视图切换
    pub is_class: bool,
    /// class 文件版本信息（如 "Java 21 (class 65.0)"）
    pub class_info: Option<String>,
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
            is_class: false,
            class_info: None,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
            layouter_bytecode: Box::new(highlight::make_bytecode_layouter()),
        }
    }

    /// 创建 .class 文件 tab
    pub fn new_class(
        title: impl Into<String>,
        entry_path: impl Into<String>,
        raw_bytes: Vec<u8>,
        language: Language,
    ) -> Self {
        let lang = language;
        let class_info = parse_class_version(&raw_bytes);
        let bytecode = match crate::java::bytecode::disassemble(&raw_bytes) {
            Ok(text) => text,
            Err(e) => t!("editor.disassemble_error", error = e).to_string(),
        };
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled: t!("editor.decompiler_placeholder").to_string(),
            bytecode,
            raw_bytes,
            language,
            active_view: ActiveView::Hex,
            is_class: true,
            class_info,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
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
            is_class: false,
            class_info: None,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(lang)),
            layouter_bytecode: Box::new(highlight::make_bytecode_layouter()),
        }
    }

    /// 创建二进制文件 tab（直接显示 hex 视图）
    pub fn new_binary(
        title: impl Into<String>,
        entry_path: impl Into<String>,
        raw_bytes: Vec<u8>,
    ) -> Self {
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled: String::new(),
            bytecode: String::new(),
            raw_bytes,
            language: Language::Plain,
            active_view: ActiveView::Hex,
            is_class: false,
            class_info: None,
            is_modified: false,
            hex_state: HexViewState::default(),
            layouter_decompiled: Box::new(highlight::make_layouter(Language::Plain)),
            layouter_bytecode: Box::new(highlight::make_bytecode_layouter()),
        }
    }
}

/// 从 .class 文件字节解析版本信息
///
/// class 文件头：`CAFEBABE` + minor(u16) + major(u16)
fn parse_class_version(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 {
        return None;
    }
    if bytes[0..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let minor = u16::from_be_bytes([bytes[4], bytes[5]]);
    let major = u16::from_be_bytes([bytes[6], bytes[7]]);
    let java_ver = if major >= 49 {
        format!("{}", major - 44)
    } else {
        format!("1.{}", major - 44)
    };
    Some(format!("Java {java_ver} (class {major}.{minor})"))
}
