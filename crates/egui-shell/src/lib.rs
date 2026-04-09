//! egui-shell: 自定义标题栏窗口壳子
//!
//! 封装 `decorations=false` 无框窗口 + 自绘标题栏 + 跨平台 resize +
//! Windows DWM 圆角。业务层只需实现 `AppContent` trait。
//!
//! @author sky

mod app;
mod codicon;
mod fonts;
mod platform;
mod titlebar;

pub use app::{run, AppContent, ShellOptions, ShellTheme};
pub use codicon::family as codicon_family;
