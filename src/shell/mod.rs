//! 窗口壳子基建
//!
//! 封装无框窗口、标题栏、平台适配、字体加载等基础设施，
//! 业务层只需实现 `AppContent` trait。
//!
//! @author sky

pub mod app;
pub mod codicon;
pub mod fonts;
pub mod platform;
pub mod theme;
pub mod titlebar;

pub use app::{run, AppContent, ShellOptions};
