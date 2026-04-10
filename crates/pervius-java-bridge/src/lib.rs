//! Java bridge：JAR 归档读取、Vineflower 反编译、字节码反汇编、class 编辑写回
//!
//! 提供与 Java 工具链交互的全部逻辑，无 UI 依赖。
//!
//! @author sky

use std::path::PathBuf;

pub mod bytecode;
pub mod class_structure;
pub mod classforge;
pub mod decompiler;
pub mod error;
pub mod jar;
pub mod process;
pub mod save;

/// 在 exe 同目录查找匹配的 JAR 文件
///
/// `prefix` — 文件名前缀（如 `"vineflower"`）
/// `filter` — 额外过滤条件（如排除 `-slim`），无需额外过滤时传 `|_| true`
pub fn find_jar(
    prefix: &str,
    filter: impl Fn(&str) -> bool,
) -> Result<PathBuf, error::BridgeError> {
    let exe = std::env::current_exe()?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| error::BridgeError::JarNotFound {
            prefix: prefix.to_string(),
            dir: PathBuf::new(),
        })?;
    let entries = std::fs::read_dir(exe_dir)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(prefix) && name.ends_with(".jar") && filter(&name) {
            return Ok(entry.path());
        }
    }
    Err(error::BridgeError::JarNotFound {
        prefix: prefix.to_string(),
        dir: exe_dir.to_path_buf(),
    })
}

/// 从 JAR 文件名解析版本号（去掉前缀和 `.jar` 后缀）
///
/// 如 `vineflower-1.11.1.jar` → `Some("1.11.1")`
pub fn jar_version(prefix: &str, path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let version = stem.strip_prefix(prefix)?.strip_prefix('-')?;
    Some(version.to_string())
}
