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
