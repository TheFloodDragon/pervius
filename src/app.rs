//! 应用核心：业务状态 + 业务逻辑调度
//!
//! @author sky

/// 轮询 `mpsc::Receiver`：有数据则返回，空或断开时执行对应分支
macro_rules! poll_recv {
    ($rx:expr, miss => $miss:expr, disconnect => $dc:expr) => {
        match $rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => $miss,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => $dc,
        }
    };
}

mod confirm;
mod decompile;
mod export;
mod handler;
mod platform;
mod tab;

pub use confirm::ConfirmAction;

use crate::settings::Settings;
use crate::ui::layout::Layout;
use egui_notify::Toasts;
use egui_shell::components::SettingsFile;
use pervius_java_bridge::decompiler::{CachedSource, DecompileTask};
use pervius_java_bridge::error::BridgeError;
use pervius_java_bridge::jar::JarArchive;
use std::collections::HashSet;
use std::sync::mpsc;

/// 应用核心业务状态
pub struct App {
    /// UI 布局状态
    pub layout: Layout,
    /// 应用设置
    pub settings: Settings,
    /// 通知提示
    pub toasts: Toasts,
    /// 当前打开的 JAR 归档
    pub(crate) jar: Option<JarArchive>,
    /// 后台加载中的 JAR
    pub(crate) loading: Option<handler::LoadingState>,
    /// 后台反编译任务
    pub(crate) decompiling: Option<DecompileTask>,
    /// 已反编译的类集合（None = 全部已反编译，Some = 仅集合内的类已完成）
    pub(crate) decompiled_classes: Option<HashSet<String>>,
    /// 待确认的破坏性动作
    pub pending_confirm: Option<ConfirmAction>,
    /// 单文件反编译结果接收队列（支持并发多个）
    pub(crate) pending_decompiles: Vec<(String, mpsc::Receiver<Result<CachedSource, BridgeError>>)>,
    /// 后台重反编译启动中（清缓存 + start 在子线程）
    pub(crate) pending_re_decompile:
        Option<(String, mpsc::Receiver<Result<DecompileTask, BridgeError>>)>,
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::load();
        Self {
            layout: Layout::new(&settings),
            settings,
            toasts: Toasts::default(),
            jar: None,
            loading: None,
            decompiling: None,
            decompiled_classes: None,
            pending_confirm: None,
            pending_decompiles: Vec::new(),
            pending_re_decompile: None,
        }
    }

    /// 打开设置对话框
    pub fn open_settings(&mut self) {
        self.layout.settings_panel.open(&self.settings);
    }
}