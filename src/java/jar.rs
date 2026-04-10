//! JAR/ZIP 归档读取
//!
//! @author sky

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
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
    /// 原始文件路径
    pub path: PathBuf,
    /// 文件内容 SHA-256（hex，前 16 字符用于缓存目录）
    pub hash: String,
    /// 条目路径 → 原始字节
    entries: HashMap<String, Vec<u8>>,
    /// 已修改但未落盘的条目路径
    modified_entries: HashSet<String>,
}

impl JarArchive {
    /// 带进度回报的打开方法（供后台线程调用）
    pub fn open_with_progress(path: &Path, progress: &LoadProgress) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(&data));
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
        Ok(Self {
            name,
            path: path.to_path_buf(),
            hash,
            entries,
            modified_entries: HashSet::new(),
        })
    }

    /// 获取条目内容
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.entries.get(path).map(|v| v.as_slice())
    }

    /// 更新条目内容（写回内存，标记为已修改）
    pub fn put(&mut self, path: &str, data: Vec<u8>) {
        self.modified_entries.insert(path.to_string());
        self.entries.insert(path.to_string(), data);
    }

    /// 是否有任何已修改但未落盘的条目
    pub fn has_modified_entries(&self) -> bool {
        !self.modified_entries.is_empty()
    }

    /// 清除所有已修改标记（放弃变更时调用）
    pub fn clear_modified(&mut self) {
        self.modified_entries.clear();
    }

    /// 已修改条目路径集合（只读引用）
    pub fn modified_entry_paths(&self) -> &HashSet<String> {
        &self.modified_entries
    }

    /// 获取排序后的条目路径列表
    pub fn paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.entries.keys().map(|s| s.as_str()).collect();
        paths.sort_unstable();
        paths
    }

    /// .class 文件条目数量
    pub fn class_count(&self) -> u32 {
        self.entries
            .keys()
            .filter(|k| k.ends_with(".class"))
            .count() as u32
    }
}
