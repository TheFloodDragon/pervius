//! 编辑器 Tab 数据结构
//!
//! @author sky

use super::highlight::{self, Language, Span};
use super::view_toggle::ActiveView;
use crate::java::class_structure::ClassStructure;
use egui_hex_view::HexViewState;
use rust_i18n::t;

/// 预处理后的代码视图数据（虚拟滚动用）
pub struct CodeData {
    /// 语法高亮 span（全文字节偏移，已排序）
    pub spans: Vec<Span>,
    /// 每行在源码中的起始字节偏移
    pub line_starts: Vec<usize>,
}

impl CodeData {
    pub fn new(source: &str, spans: Vec<Span>) -> Self {
        let line_starts = highlight::compute_line_starts(source);
        Self { spans, line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

/// 字节码面板导航选中状态
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BytecodeSelection {
    ClassInfo,
    Field(usize),
    Method(usize),
}

/// 编辑器 Tab 数据
pub struct EditorTab {
    pub title: String,
    /// JAR 内条目路径（用于 tab 去重 + widget ID）
    pub entry_path: Option<String>,
    /// 反编译源码（只读）
    pub decompiled: String,
    /// 原始字节（只读，用于 hex 视图）
    pub raw_bytes: Vec<u8>,
    pub language: Language,
    pub active_view: ActiveView,
    /// .class 文件才显示三视图切换
    pub is_class: bool,
    /// 纯文本文件（可直接编辑）
    pub is_text: bool,
    /// class 文件版本信息（如 "Java 21 (class 65.0)"）
    pub class_info: Option<String>,
    pub is_modified: bool,
    /// Hex 视图交互状态
    pub hex_state: HexViewState,
    /// 反编译视图预处理数据
    pub(super) decompiled_data: CodeData,
    /// 反编译行 → 原始源码行号映射（1-indexed），None 表示无映射
    pub decompiled_line_mapping: Vec<Option<u32>>,
    /// 解析后的 class 结构化数据
    pub class_structure: Option<ClassStructure>,
    /// 字节码面板当前选中项
    pub bc_selection: BytecodeSelection,
    /// 字节码面板左侧导航栏宽度（可拖拽调整）
    pub nav_width: f32,
}

impl EditorTab {
    /// 创建 .class 文件 tab
    pub fn new_class(
        title: impl Into<String>,
        entry_path: impl Into<String>,
        raw_bytes: Vec<u8>,
        language: Language,
    ) -> Self {
        let cs = crate::java::bytecode::disassemble(&raw_bytes).ok();
        let class_info = cs.as_ref().map(|c| c.info.version.clone());
        let bc_selection = cs
            .as_ref()
            .map(|c| {
                if c.methods.is_empty() {
                    BytecodeSelection::ClassInfo
                } else {
                    BytecodeSelection::Method(0)
                }
            })
            .unwrap_or(BytecodeSelection::ClassInfo);
        let decompiled = t!("editor.decompiler_placeholder").to_string();
        let dec_data = CodeData::new(&decompiled, highlight::compute_spans(&decompiled, language));
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled,
            raw_bytes,
            language,
            active_view: ActiveView::Hex,
            is_class: true,
            is_text: false,
            class_info,
            is_modified: false,
            hex_state: HexViewState::default(),
            decompiled_data: dec_data,
            decompiled_line_mapping: Vec::new(),
            class_structure: cs,
            bc_selection,
            nav_width: 220.0,
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
        let dec_data = CodeData::new(&text, highlight::compute_spans(&text, language));
        Self {
            title: title.into(),
            entry_path: Some(entry_path.into()),
            decompiled: text,
            raw_bytes,
            language,
            active_view: ActiveView::Decompiled,
            is_class: false,
            is_text: true,
            class_info: None,
            is_modified: false,
            hex_state: HexViewState::default(),
            decompiled_data: dec_data,
            decompiled_line_mapping: Vec::new(),
            class_structure: None,
            bc_selection: BytecodeSelection::ClassInfo,
            nav_width: 220.0,
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
            raw_bytes,
            language: Language::Plain,
            active_view: ActiveView::Hex,
            is_class: false,
            is_text: false,
            class_info: None,
            is_modified: false,
            hex_state: HexViewState::default(),
            decompiled_data: CodeData::new("", vec![]),
            decompiled_line_mapping: Vec::new(),
            class_structure: None,
            bc_selection: BytecodeSelection::ClassInfo,
            nav_width: 220.0,
        }
    }

    /// 更新反编译源码（反编译完成后调用）
    pub fn set_decompiled(
        &mut self,
        source: String,
        lang: Language,
        line_mapping: Vec<Option<u32>>,
    ) {
        self.decompiled_data = CodeData::new(&source, highlight::compute_spans(&source, lang));
        self.decompiled = source;
        self.language = lang;
        self.decompiled_line_mapping = line_mapping;
    }

    /// 当前选中方法的字节码文本（find bar 用）
    pub fn selected_bytecode_text(&self) -> &str {
        if let (Some(cs), BytecodeSelection::Method(idx)) =
            (&self.class_structure, self.bc_selection)
        {
            cs.methods
                .get(idx)
                .map(|m| m.bytecode.as_str())
                .unwrap_or("")
        } else {
            ""
        }
    }

    /// 重建反编译/文本视图的高亮数据（文本编辑后调用）
    pub fn refresh_decompiled_data(&mut self) {
        self.decompiled_data = CodeData::new(
            &self.decompiled,
            highlight::compute_spans(&self.decompiled, self.language),
        );
    }
}
