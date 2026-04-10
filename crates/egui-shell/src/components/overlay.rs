//! 浮动覆盖层组件：窗口、确认弹窗
//!
//! @author sky

pub mod confirm;
pub mod window;

pub use confirm::{ConfirmDialog, ConfirmResult, ConfirmTheme};
pub use window::FloatingWindow;
