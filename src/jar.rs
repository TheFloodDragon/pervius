//! JAR/ZIP 归档读取
//!
//! @author sky

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

/// JAR 加载进度（跨线程共享）
pub struct LoadProgress {
    pub current: AtomicU32,
    pub total: AtomicU32,
}

impl LoadProgress {
    pub fn new() -> Self {
        Self {
            current: AtomicU32::new(0),
            total: AtomicU32::new(0),
        }
    }
}

/// JAR 归档内存表示
pub struct JarArchive {
    /// 归档文件名
    pub name: String,
    /// 条目路径 → 原始字节
    entries: HashMap<String, Vec<u8>>,
}

impl JarArchive {
    /// 从文件路径打开 JAR/ZIP（同步，阻塞调用方）
    pub fn open(path: &Path) -> Result<Self, String> {
        let progress = LoadProgress::new();
        Self::open_with_progress(path, &progress)
    }

    /// 带进度回报的打开方法（供后台线程调用）
    pub fn open_with_progress(path: &Path, progress: &LoadProgress) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let cursor = std::io::Cursor::new(data);
        let mut zip = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;
        let total = zip.len();
        progress.total.store(total as u32, Ordering::Relaxed);
        let mut entries = HashMap::new();
        for i in 0..total {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            if !entry.is_dir() {
                let name = entry.name().to_owned();
                let mut buf = Vec::with_capacity(entry.size() as usize);
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                entries.insert(name, buf);
            }
            progress.current.store((i + 1) as u32, Ordering::Relaxed);
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self { name, entries })
    }

    /// 获取条目内容
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.entries.get(path).map(|v| v.as_slice())
    }

    /// 获取排序后的条目路径列表
    pub fn paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        paths.sort_unstable();
        paths
    }
}
