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
    /// 循环切换到下一个视图
    pub fn next(self) -> Self {
        match self {
            Self::Decompiled => Self::Bytecode,
            Self::Bytecode => Self::Hex,
            Self::Hex => Self::Decompiled,
        }
    }
}
