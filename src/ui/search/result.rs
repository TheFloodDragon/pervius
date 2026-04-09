//! 搜索结果数据结构
//!
//! @author sky

/// 按文件分组的搜索结果
pub struct SearchResultGroup {
    /// 类名（如 "MinecraftServer"）
    pub class_name: String,
    /// 包路径（如 "net.minecraft.server"）
    pub package: String,
    /// 该文件内的匹配列表
    pub matches: Vec<SearchMatch>,
    /// 分组是否展开
    pub expanded: bool,
}

/// 单条匹配（同时持有 decompiled 和 bytecode 两种视图的预览）
pub struct SearchMatch {
    /// 上下文位置（方法名或行号，如 "loadWorlds()" 或 "line 142"）
    pub location: String,
    /// 反编译视图预览
    pub decompiled: SourcePreview,
    /// 字节码视图预览
    pub bytecode: SourcePreview,
}

/// 某一视图模式下的预览数据
pub struct SourcePreview {
    /// 结果列表中显示的匹配行摘要
    pub preview: String,
    /// preview 中高亮区间 (start_byte, end_byte)
    pub highlight_ranges: Vec<(usize, usize)>,
    /// 预览面板完整源码行
    pub source_lines: Vec<String>,
    /// 匹配行在 source_lines 中的索引（0-based）
    pub match_line: usize,
}
