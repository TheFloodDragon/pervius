//! 反编译源码导出
//!
//! @author sky

use super::App;
use pervius_java_bridge::decompiler;
use rust_i18n::t;
use std::path::Path;

impl App {
    /// 导出反编译源码到用户选择的目录
    ///
    /// 将 Vineflower 缓存目录中的 `.java` / `.kt` 文件复制到目标目录，
    /// 保持原始包结构。
    pub fn export_decompiled(&mut self) {
        let Some(jar) = &self.jar else {
            self.toasts.warning(t!("layout.export_no_jar"));
            return;
        };
        if self.decompiling.is_some() {
            self.toasts.warning(t!("layout.decompile_in_progress"));
            return;
        }
        if !decompiler::is_cached(&jar.hash) {
            self.toasts.warning(t!("layout.export_not_decompiled"));
            return;
        }
        let cache = match decompiler::cache_dir(&jar.hash) {
            Ok(d) if d.exists() => d,
            _ => {
                self.toasts.warning(t!("layout.export_not_decompiled"));
                return;
            }
        };
        let Some(dest) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        match copy_sources(&cache, &dest) {
            Ok(count) => {
                let display = dest.to_string_lossy();
                self.toasts
                    .info(t!("layout.export_complete", path = display, count = count));
                log::info!("Exported {count} files to {display}");
            }
            Err(e) => {
                self.toasts.error(t!("layout.export_failed", error = e));
                log::error!("Export failed: {e}");
            }
        }
    }
}

/// 递归复制 `.java` / `.kt` 源码文件到目标目录，保持目录结构
///
/// 跳过 `.complete` 等非源码文件，返回复制的文件数。
fn copy_sources(src: &Path, dest: &Path) -> Result<usize, String> {
    let mut count = 0;
    copy_sources_recursive(src, src, dest, &mut count)?;
    Ok(count)
}

/// 递归遍历源目录，复制匹配的源码文件
fn copy_sources_recursive(
    root: &Path,
    dir: &Path,
    dest: &Path,
    count: &mut usize,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{e}"))?;
        let path = entry.path();
        if path.is_dir() {
            copy_sources_recursive(root, &path, dest, count)?;
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "java" && ext != "kt" {
                continue;
            }
            let rel = path.strip_prefix(root).map_err(|e| format!("{e}"))?;
            let target = dest.join(rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
            }
            std::fs::copy(&path, &target).map_err(|e| format!("{e}"))?;
            *count += 1;
        }
    }
    Ok(())
}
