//! Java 子进程管理：构建、启动、终止
//!
//! 统一 java 定位、`-jar` 参数、Windows `CREATE_NO_WINDOW` 和管道配置，
//! 所有 Java 外部工具（Vineflower、ClassForge 等）通过 `JavaCommand` 启动。
//!
//! @author sky

use crate::error::BridgeError;
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

/// 从 JAVA_HOME 环境变量定位 java 可执行文件
pub fn find_java() -> Result<PathBuf, BridgeError> {
    let java_home = std::env::var("JAVA_HOME").map_err(|_| BridgeError::JavaHomeNotSet)?;
    let java_home = PathBuf::from(java_home);
    let java = if cfg!(windows) {
        java_home.join("bin").join("java.exe")
    } else {
        java_home.join("bin").join("java")
    };
    if !java.exists() {
        return Err(BridgeError::JavaNotFound(java));
    }
    Ok(java)
}

/// Java `-jar` 子进程构建器
///
/// 自动处理 java 定位、Windows `CREATE_NO_WINDOW`、管道配置。
pub struct JavaCommand {
    cmd: Command,
}

impl JavaCommand {
    /// 创建 `java -jar <jar>` 命令
    pub fn new(jar: &Path) -> Result<Self, BridgeError> {
        let java = find_java()?;
        let mut cmd = Command::new(java);
        cmd.arg("-jar").arg(jar);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        Ok(Self { cmd })
    }

    /// 追加单个参数
    pub fn arg(&mut self, arg: impl AsRef<std::ffi::OsStr>) -> &mut Self {
        self.cmd.arg(arg);
        self
    }

    /// 同步执行，管道收集 stdout/stderr
    pub fn output(&mut self) -> Result<Output, std::io::Error> {
        self.cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        self.cmd.output()
    }

    /// 启动子进程（stdout/stderr 管道）
    pub fn spawn(&mut self) -> Result<Child, std::io::Error> {
        self.cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        self.cmd.spawn()
    }

    /// 启动子进程（stdin/stdout/stderr 管道）
    pub fn spawn_with_stdin(&mut self) -> Result<Child, std::io::Error> {
        self.cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.cmd.spawn()
    }
}

/// 终止进程及其子进程树
#[cfg(windows)]
pub fn kill_tree(pid: u32) {
    let mut cmd = Command::new("taskkill");
    cmd.args(["/F", "/T", "/PID", &pid.to_string()]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let _ = cmd.output();
}

/// 终止进程及其子进程树
#[cfg(not(windows))]
pub fn kill_tree(pid: u32) {
    let _ = Command::new("kill").arg(pid.to_string()).output();
}
