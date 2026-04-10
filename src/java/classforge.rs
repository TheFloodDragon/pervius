//! ClassForge 集成：调用 ASM 处理字节码修改、StackMapTable 生成、max 值计算
//!
//! 两种模式：
//! - `patch_methods`: 发送 class 字节 + 方法编辑列表，ASM 替换字节码并重算帧
//! - `recompute_frames`: 仅重算 StackMapTable / max_stack / max_locals
//!
//! @author sky

use std::io::Write;
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::decompiler;

/// 定位 classforge.jar（exe 同目录，匹配 classforge*.jar）
fn find_classforge() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| "cannot determine exe directory".to_string())?;
    let entries = std::fs::read_dir(exe_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("classforge") && name.ends_with(".jar") {
            return Ok(entry.path());
        }
    }
    Err(format!("classforge.jar not found in {}", exe_dir.display()))
}

/// 方法字节码编辑
pub struct MethodEdit {
    /// 方法名
    pub name: String,
    /// 方法描述符
    pub descriptor: String,
    /// 新字节码文本（Pervius 格式）
    pub bytecode: String,
}

/// 调用 classforge --patch 模式：替换方法字节码 + 重算帧/max 值
///
/// ASM 接管常量池管理、指令编码、StackMapTable 生成、max_stack/max_locals 计算。
/// Rust 侧无需维护 CP，只传字节码文本。
///
/// 协议（stdin, big-endian）：
///   [4B class 长度][class 字节]
///   [4B 编辑数]
///   每条：[2B name 长度][name][2B desc 长度][desc][4B code 长度][code 文本]
pub fn patch_methods(
    class_bytes: &[u8],
    edits: &[MethodEdit],
    jar_path: Option<&Path>,
) -> Result<Vec<u8>, String> {
    let java = decompiler::find_java()?;
    let classforge = find_classforge()?;
    let mut cmd = Command::new(&java);
    cmd.arg("-jar").arg(&classforge).arg("--patch");
    if let Some(path) = jar_path {
        cmd.arg("--classpath").arg(path);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("classforge spawn failed: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // class 数据
        stdin
            .write_all(&(class_bytes.len() as u32).to_be_bytes())
            .map_err(|e| format!("stdin write failed: {e}"))?;
        stdin
            .write_all(class_bytes)
            .map_err(|e| format!("stdin write failed: {e}"))?;
        // 编辑数
        stdin
            .write_all(&(edits.len() as u32).to_be_bytes())
            .map_err(|e| format!("stdin write failed: {e}"))?;
        for edit in edits {
            write_prefixed_string(&mut stdin, &edit.name)?;
            write_prefixed_string(&mut stdin, &edit.descriptor)?;
            write_prefixed_string_u32(&mut stdin, &edit.bytecode)?;
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("classforge wait failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("classforge failed: {stderr}"));
    }
    if output.stdout.is_empty() {
        return Err("classforge produced no output".to_string());
    }
    log::debug!(
        "classforge patch: {} -> {} bytes ({} methods patched)",
        class_bytes.len(),
        output.stdout.len(),
        edits.len()
    );
    Ok(output.stdout)
}

/// 写入 u16 长度前缀的 UTF-8 字符串
fn write_prefixed_string(w: &mut dyn Write, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u16).to_be_bytes())
        .map_err(|e| format!("stdin write failed: {e}"))?;
    w.write_all(bytes)
        .map_err(|e| format!("stdin write failed: {e}"))
}

/// 写入 u32 长度前缀的 UTF-8 字符串（用于较长的字节码文本）
fn write_prefixed_string_u32(w: &mut dyn Write, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|e| format!("stdin write failed: {e}"))?;
    w.write_all(bytes)
        .map_err(|e| format!("stdin write failed: {e}"))
}
