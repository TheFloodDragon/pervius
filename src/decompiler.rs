//! Vineflower 反编译器集成：Java 检测、反编译调用、磁盘缓存
//!
//! @author sky

use std::io::BufRead;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};

/// 反编译进度（跨线程共享）
pub struct DecompileProgress {
    pub current: AtomicU32,
    pub total: AtomicU32,
}

/// 后台反编译任务句柄
pub struct DecompileTask {
    pub jar_name: String,
    pub progress: Arc<DecompileProgress>,
    pub receiver: mpsc::Receiver<Result<(), String>>,
}

/// 从 JAVA_HOME 环境变量定位 java 可执行文件
pub fn find_java() -> Result<PathBuf, String> {
    let java_home = std::env::var("JAVA_HOME")
        .map_err(|_| "JAVA_HOME environment variable is not set".to_string())?;
    let java_home = PathBuf::from(java_home);
    let java = if cfg!(windows) {
        java_home.join("bin").join("java.exe")
    } else {
        java_home.join("bin").join("java")
    };
    if !java.exists() {
        return Err(format!("Java executable not found at {}", java.display()));
    }
    Ok(java)
}

/// 定位 vineflower JAR（exe 同目录，匹配 vineflower-*.jar）
pub fn find_vineflower() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe.parent().ok_or("Cannot determine exe directory")?;
    let entries = std::fs::read_dir(exe_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("vineflower") && name.ends_with(".jar") && !name.contains("-slim") {
            return Ok(entry.path());
        }
    }
    Err(format!(
        "vineflower*.jar not found in {}",
        exe_dir.display()
    ))
}

/// 获取缓存根目录
fn cache_root() -> Result<PathBuf, String> {
    let base = dirs::cache_dir().ok_or("Cannot determine cache directory")?;
    Ok(base.join("pervius").join("decompiled"))
}

/// 获取指定 JAR 哈希的缓存目录
pub fn cache_dir(hash: &str) -> Result<PathBuf, String> {
    let prefix = &hash[..16.min(hash.len())];
    Ok(cache_root()?.join(prefix))
}

/// 检查缓存是否完整（存在 .complete 标记文件）
pub fn is_cached(hash: &str) -> bool {
    cache_dir(hash)
        .ok()
        .map(|d| d.join(".complete").exists())
        .unwrap_or(false)
}

/// 清除指定 JAR 的反编译缓存
pub fn clear_cache(hash: &str) {
    if let Ok(dir) = cache_dir(hash) {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// 缓存查找结果
pub struct CachedSource {
    pub source: String,
    pub is_kotlin: bool,
}

/// 获取缓存源码文件路径（不读取内容）
pub fn cached_source_path(hash: &str, class_path: &str) -> Option<std::path::PathBuf> {
    let dir = cache_dir(hash).ok()?;
    let base = class_to_base_path(class_path);
    for ext in &[".java", ".kt"] {
        let file = dir.join(format!("{base}{ext}"));
        log::debug!("cached_source_path: trying {}", file.display());
        if file.exists() {
            return Some(file);
        }
    }
    None
}

/// 从缓存读取反编译源码
///
/// class_path 形如 `com/example/Foo.class` 或 `com/example/Foo$Bar.class`，
/// 自动映射到外层类的 .java 或 .kt 文件（内部类源码在外层类文件中）。
pub fn cached_source(hash: &str, class_path: &str) -> Option<CachedSource> {
    let dir = cache_dir(hash).ok()?;
    let base = class_to_base_path(class_path);
    for (ext, is_kt) in &[(".java", false), (".kt", true)] {
        let file = dir.join(format!("{base}{ext}"));
        if let Ok(src) = std::fs::read_to_string(&file) {
            log::debug!("Cache hit: {}", file.display());
            return Some(CachedSource {
                source: src,
                is_kotlin: *is_kt,
            });
        }
    }
    log::warn!("Cache miss: {class_path} (tried .java and .kt)");
    None
}

/// .class 条目路径 → 去掉扩展名和内部类后缀的基础路径
///
/// `com/example/Foo$Bar$1.class` → `com/example/Foo`
fn class_to_base_path(class_path: &str) -> &str {
    let without_ext = class_path.strip_suffix(".class").unwrap_or(class_path);
    match without_ext.find('$') {
        Some(pos) => &without_ext[..pos],
        None => without_ext,
    }
}

/// 启动后台反编译任务
///
/// 在后台线程中运行 `java -jar vineflower.jar <jar_path> <cache_dir>`，
/// 逐行解析 stdout 更新进度，完成后通过 channel 发送结果。
pub fn start(
    jar_path: &Path,
    jar_name: &str,
    hash: &str,
    class_count: u32,
) -> Result<DecompileTask, String> {
    let java = find_java()?;
    let vineflower = find_vineflower()?;
    let output_dir = cache_dir(hash)?;
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let progress = Arc::new(DecompileProgress {
        current: AtomicU32::new(0),
        total: AtomicU32::new(class_count),
    });
    let (tx, rx) = mpsc::channel();
    let jar_path = jar_path.to_path_buf();
    let p = progress.clone();
    std::thread::spawn(move || {
        let result = run_vineflower(&java, &vineflower, &jar_path, &output_dir, &p);
        let _ = tx.send(result);
    });
    Ok(DecompileTask {
        jar_name: jar_name.to_string(),
        progress,
        receiver: rx,
    })
}

/// 执行 vineflower 进程并解析进度
fn run_vineflower(
    java: &Path,
    vineflower: &Path,
    jar_path: &Path,
    output_dir: &Path,
    progress: &DecompileProgress,
) -> Result<(), String> {
    let mut cmd = std::process::Command::new(java);
    cmd.arg("-jar")
        .arg(vineflower)
        .arg(jar_path)
        .arg(output_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn vineflower: {e}"))?;
    // 逐行读 stderr，匹配 "Decompiling class" 更新进度，其余行记日志
    if let Some(stderr) = child.stderr.take() {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.contains("Decompiling class") {
                progress.current.fetch_add(1, Ordering::Relaxed);
            } else if line.contains("ERROR") || line.contains("Exception") {
                log::error!("vineflower: {line}");
            } else {
                log::debug!("vineflower: {line}");
            }
        }
    }
    let status = child
        .wait()
        .map_err(|e| format!("Vineflower process error: {e}"))?;
    if !status.success() {
        return Err(format!("Vineflower exited with code {:?}", status.code()));
    }
    // 写入完成标记
    let _ = std::fs::write(output_dir.join(".complete"), "");
    Ok(())
}
