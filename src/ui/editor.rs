//! 编辑器模块
//!
//! @author sky

mod area;
mod bytecode;
mod render;
pub mod style;
mod tab;
pub mod view_toggle;
mod viewer;

pub use area::EditorArea;
pub use tab::EditorTab;
pub use viewer::TabAction;
