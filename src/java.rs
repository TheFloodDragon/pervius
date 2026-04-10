//! Java 相关：JAR 归档读取、Vineflower 反编译、字节码反汇编
//!
//! @author sky

use std::path::PathBuf;

pub mod bytecode;
pub mod class_structure;
pub mod classforge;
pub mod decompiler;
pub mod jar;
pub mod process;
pub mod save;

/// 在 exe 同目录查找匹配的 JAR 文件
///
/// `prefix` — 文件名前缀（如 `"vineflower"`）
/// `filter` — 额外过滤条件（如排除 `-slim`），无需额外过滤时传 `|_| true`
pub fn find_jar(prefix: &str, filter: impl Fn(&str) -> bool) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| "cannot determine exe directory".to_string())?;
    let entries = std::fs::read_dir(exe_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(prefix) && name.ends_with(".jar") && filter(&name) {
            return Ok(entry.path());
        }
    }
    Err(format!("{prefix}*.jar not found in {}", exe_dir.display()))
}

/// 从 JAR 文件名解析版本号（去掉前缀和 `.jar` 后缀）
///
/// 如 `vineflower-1.11.1.jar` → `Some("1.11.1")`
pub fn jar_version(prefix: &str, path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let version = stem.strip_prefix(prefix)?.strip_prefix('-')?;
    Some(version.to_string())
}
