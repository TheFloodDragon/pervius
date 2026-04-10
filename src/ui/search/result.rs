//! 搜索结果数据结构
//!
//! @author sky

/// 按类分组的搜索结果
pub struct SearchResultGroup {
    /// 类名（如 "MyClass"）
    pub class_name: String,
    /// 包路径（如 "com.example"）
    pub package: String,
    /// JAR 内条目路径（用于打开 tab）
    pub entry_path: String,
    /// 该条目在 SearchIndex.entries 中的下标（用于预览面板取完整源码）
    pub source_index: usize,
    /// 该类内的匹配行列表
    pub matches: Vec<LineMatch>,
    /// 分组是否展开
    pub expanded: bool,
}

/// 单行匹配
pub struct LineMatch {
    /// 匹配行号（0-based）
    pub line: usize,
    /// 匹配行文本（去前导空白）
    pub preview: String,
    /// preview 内高亮区间列表 (start_byte, end_byte)
    pub highlights: Vec<(usize, usize)>,
}
