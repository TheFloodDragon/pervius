//! egui-editor: 语法高亮代码查看器 + 查找栏
//!
//! 提供通用的代码视图渲染（行号栏 + 语法高亮 + 搜索匹配）、
//! 多语言语法高亮引擎（tree-sitter）、全文搜索算法和浮动查找栏组件。
//!
//! @author sky

pub mod code_view;
pub mod find_bar;
pub mod highlight;
pub mod search;
pub mod theme;
mod viewport;

pub use code_view::{EditableLayoutCache, LayoutCache};
pub use find_bar::FindBar;
pub use highlight::{Language, Span, TokenKind};
pub use search::FindMatch;
pub use theme::{CodeViewTheme, FindBarTheme, SyntaxTheme};
