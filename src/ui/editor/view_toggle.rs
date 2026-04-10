//! Decompiled / Bytecode / Hex 三视图切换
//!
//! @author sky

/// 活跃视图
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Decompiled,
    Bytecode,
    Hex,
}

impl ActiveView {
    /// 在 Decompiled 和 Bytecode 之间切换（不含 Hex）
    pub fn next(self) -> Self {
        match self {
            Self::Decompiled => Self::Bytecode,
            Self::Bytecode => Self::Decompiled,
            Self::Hex => Self::Decompiled,
        }
    }
}
