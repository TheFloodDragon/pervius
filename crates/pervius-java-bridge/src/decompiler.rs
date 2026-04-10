//! Vineflower 反编译器集成：Java 检测、反编译调用、磁盘缓存
//!
//! @author sky

use crate::error::BridgeError;
use std::collections::HashSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use crate::process;

/// 反编译进度（跨线程共享）
pub struct DecompileProgress {
    /// Decompiling 阶段已处理类数
    pub current: AtomicU32,
    /// Preprocessing 阶段统计的根类总数（动态累加）
    pub total: AtomicU32,
    /// 已完成反编译的类路径及其文件夹前缀
    pub decompiled: Mutex<HashSet<String>>,
}

/// 后台反编译任务句柄
///
/// Drop 时自动终止正在运行的 vineflower 子进程。
pub struct DecompileTask {
    pub jar_name: String,
    pub progress: Arc<DecompileProgress>,
    pub receiver: mpsc::Receiver<Result<(), BridgeError>>,
    /// 正在运行的子进程 PID（0 表示未运行或已结束）
    child_pid: Arc<AtomicU32>,
}

impl Drop for DecompileTask {
    fn drop(&mut self) {
        let pid = self.child_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return;
        }
        log::info!("Killing vineflower process (PID {pid})");
        process::kill_tree(pid);
    }
}

/// 获取 Vineflower 版本号（从 JAR 文件名解析）
///
/// 返回版本字符串（如 `"1.11.1"`），未找到时返回 `None`。
/// 调用方负责 i18n 格式化。
pub fn vineflower_version() -> Option<String> {
    let path = find_vineflower().ok()?;
    crate::jar_version("vineflower", &path)
}

/// 定位 vineflower JAR（exe 同目录，匹配 vineflower-*.jar，排除 -slim）
pub fn find_vineflower() -> Result<PathBuf, BridgeError> {
    crate::find_jar("vineflower", |name| !name.contains("-slim"))
}

/// 获取缓存根目录
fn cache_root() -> Result<PathBuf, BridgeError> {
    let base = dirs::cache_dir().ok_or(BridgeError::NoCacheDir)?;
    Ok(base.join("pervius").join("decompiled"))
}

/// 获取指定 JAR 哈希的缓存目录
pub fn cache_dir(hash: &str) -> Result<PathBuf, BridgeError> {
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
#[derive(Clone)]
pub struct CachedSource {
    /// 反编译后的源码（已清理行号标记）
    pub source: String,
    /// 是否为 Kotlin 类
    pub is_kotlin: bool,
    /// 反编译行 → 原始源码行号（1-indexed），None 表示无映射
    pub line_mapping: Vec<Option<u32>>,
}

/// 获取缓存源码文件路径（不读取内容）
pub fn cached_source_path(hash: &str, class_path: &str) -> Option<PathBuf> {
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
    let result = try_read_source(&dir, base);
    if result.is_none() {
        log::warn!("Cache miss: {class_path} (tried .java and .kt)");
    }
    result
}

/// 从 Vineflower 输出中提取行号标记并移除
///
/// Vineflower 在 `--bytecode-source-mapping=1 --__dump_original_lines__=1` 模式下
/// 会在行尾追加 `// <line_number> [<line_number> ...]` 注释。
/// 返回 (清理后的源码, 每行对应的原始行号映射)。
fn strip_line_markers(raw: &str) -> (String, Vec<Option<u32>>) {
    let mut cleaned = String::with_capacity(raw.len());
    let mut mapping = Vec::new();
    for line in raw.lines() {
        let (text, line_no) = extract_line_marker(line);
        cleaned.push_str(text);
        cleaned.push('\n');
        mapping.push(line_no);
    }
    // 移除末尾多余换行
    if cleaned.ends_with('\n') && !raw.ends_with('\n') {
        cleaned.pop();
    }
    (cleaned, mapping)
}

/// 提取行尾的 Vineflower 行号标记
///
/// 匹配行尾 `// <digits>[ <digits>...]` 格式，取第一个数字作为原始行号。
/// 需要排除正常注释，仅匹配纯数字注释。
fn extract_line_marker(line: &str) -> (&str, Option<u32>) {
    // 从行尾向前查找最后一个 "//"
    let Some(pos) = line.rfind("//") else {
        return (line, None);
    };
    let after = line[pos + 2..].trim();
    // 必须是纯数字（空格分隔），不包含其他字符
    if after.is_empty() {
        return (line, None);
    }
    let is_line_marker = after
        .split_whitespace()
        .all(|tok| tok.parse::<u32>().is_ok());
    if !is_line_marker {
        return (line, None);
    }
    // 取第一个数字作为行号
    let first = after.split_whitespace().next().unwrap();
    let line_no: u32 = first.parse().unwrap();
    // 移除 "//" 和之前的尾部空格
    let text = line[..pos].trim_end();
    (text, Some(line_no))
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

/// 在目录中尝试读取 .java 或 .kt 反编译源码
fn try_read_source(dir: &Path, base: &str) -> Option<CachedSource> {
    for (ext, is_kt) in &[(".java", false), (".kt", true)] {
        let file = dir.join(format!("{base}{ext}"));
        if let Ok(raw) = std::fs::read_to_string(&file) {
            log::debug!("Source found: {}", file.display());
            let (source, line_mapping) = strip_line_markers(&raw);
            return Some(CachedSource {
                source,
                is_kotlin: *is_kt,
                line_mapping,
            });
        }
    }
    None
}

/// 将子进程管道转发到 channel（在独立线程中运行）
fn pipe_to_channel(
    stream: impl std::io::Read + Send + 'static,
    tx: mpsc::Sender<String>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stream);
        for line in reader.lines().flatten() {
            if tx.send(line).is_err() {
                break;
            }
        }
    })
}

/// 启动后台反编译任务
///
/// 在后台线程中运行 `java -jar vineflower.jar <jar_path> <cache_dir>`，
/// 逐行解析 stdout 更新进度，完成后通过 channel 发送结果。
pub fn start(
    jar_path: &Path,
    jar_name: &str,
    hash: &str,
    _class_count: u32,
) -> Result<DecompileTask, BridgeError> {
    process::find_java()?;
    let vineflower = find_vineflower()?;
    let output_dir = cache_dir(hash)?;
    std::fs::create_dir_all(&output_dir)?;
    let progress = Arc::new(DecompileProgress {
        current: AtomicU32::new(0),
        total: AtomicU32::new(0),
        decompiled: Mutex::new(HashSet::new()),
    });
    let child_pid = Arc::new(AtomicU32::new(0));
    let (tx, rx) = mpsc::channel();
    let jar_path = jar_path.to_path_buf();
    let p = progress.clone();
    let cp = child_pid.clone();
    std::thread::spawn(move || {
        let result = run_vineflower(&vineflower, &jar_path, &output_dir, &p, &cp);
        let _ = tx.send(result);
    });
    Ok(DecompileTask {
        jar_name: jar_name.to_string(),
        progress,
        receiver: rx,
        child_pid,
    })
}

/// 执行 vineflower 进程并解析进度
fn run_vineflower(
    vineflower: &Path,
    jar_path: &Path,
    output_dir: &Path,
    progress: &DecompileProgress,
    child_pid: &AtomicU32,
) -> Result<(), BridgeError> {
    let thr = std::thread::available_parallelism()
        .map(|n| n.get().max(2) / 2)
        .unwrap_or(2);
    let mut cmd = process::JavaCommand::new(vineflower)?;
    cmd.arg("--bytecode-source-mapping=1")
        .arg("--__dump_original_lines__=1")
        .arg(format!("-thr={thr}"))
        .arg(jar_path)
        .arg(output_dir);
    let mut child = cmd.spawn().map_err(BridgeError::SpawnFailed)?;
    child_pid.store(child.id(), Ordering::Relaxed);
    // stdout 和 stderr 各起一个线程读取，汇入同一 channel 统一解析
    // Vineflower 可能把进度日志写到任一流，合并后不遗漏且避免管道满死锁
    let (line_tx, line_rx) = mpsc::channel::<String>();
    let stdout_thread = child
        .stdout
        .take()
        .map(|out| pipe_to_channel(out, line_tx.clone()));
    let stderr_thread = child
        .stderr
        .take()
        .map(|err| pipe_to_channel(err, line_tx.clone()));
    drop(line_tx);
    // 进度阶段：
    //   "Preprocessing class" — 分析阶段（每个根类一次）
    //   "Decompiling class"   — 反编译输出阶段（每个根类一次）
    // 两阶段都计入 current，total 在 Preprocessing 时动态累加（×2 留给 Decompiling）
    // "Decompiling class X" 出现时，上一个类已完成输出
    let mut prev_class: Option<String> = None;
    for line in line_rx {
        if line.contains("Preprocessing class") {
            progress.total.fetch_add(2, Ordering::Relaxed);
            progress.current.fetch_add(1, Ordering::Relaxed);
        } else if line.contains("Decompiling class") {
            progress.current.fetch_add(1, Ordering::Relaxed);
            if let Some(prev) = prev_class.take() {
                mark_decompiled(&progress.decompiled, &prev);
            }
            prev_class = extract_class_name(&line);
        } else if line.starts_with("ERROR:") {
            log::error!("vineflower: {line}");
        }
    }
    // 最后一个类
    if let Some(prev) = prev_class {
        mark_decompiled(&progress.decompiled, &prev);
    }
    for t in [stdout_thread, stderr_thread].into_iter().flatten() {
        let _ = t.join();
    }
    let status = child.wait()?;
    child_pid.store(0, Ordering::Relaxed);
    if !status.success() {
        return Err(BridgeError::ExitCode(status.code()));
    }
    // 写入完成标记
    let _ = std::fs::write(output_dir.join(".complete"), "");
    Ok(())
}

/// 从 Vineflower 日志行提取类名，统一为 `/` 分隔格式
///
/// Vineflower 日志可能输出 `com.example.Foo`（点）或 `com/example/Foo`（斜杠），
/// 统一转为斜杠格式以匹配树节点路径。
fn extract_class_name(line: &str) -> Option<String> {
    let marker = "Decompiling class ";
    let pos = line.find(marker)?;
    let name = line[pos + marker.len()..].trim();
    if name.is_empty() {
        return None;
    }
    if name.contains('/') {
        Some(name.to_string())
    } else {
        Some(name.replace('.', "/"))
    }
}

/// 将类名及其文件夹前缀加入已反编译集合
fn mark_decompiled(set: &Mutex<HashSet<String>>, class_name: &str) {
    let mut guard = set.lock().unwrap();
    guard.insert(class_name.to_string());
    for (i, _) in class_name.match_indices('/') {
        guard.insert(class_name[..i + 1].to_string());
    }
}

/// 临时目录 RAII 守卫，Drop 时自动清理
struct TempDirGuard {
    /// 目录路径
    path: PathBuf,
    /// 是否在 Drop 时删除
    should_clean: bool,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.should_clean {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// 单文件反编译：将 class 字节写入临时目录，跑 Vineflower
///
/// `class_path` 形如 `com/example/Foo.class`，用于构建临时目录结构。
/// `jar_path` 用作 Vineflower 的 context library（-e 参数），提供依赖解析。
/// `cache_hash` 传入 `Some(hash)` 时输出写入缓存目录（首次预览），传入 `None` 时输出写入临时目录（保存后重编译）。
pub fn decompile_single_class(
    bytes: &[u8],
    class_path: &str,
    jar_path: &Path,
    cache_hash: Option<&str>,
) -> Result<CachedSource, BridgeError> {
    let vineflower = find_vineflower()?;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_input = TempDirGuard {
        path: std::env::temp_dir().join(format!("pervius_single_{id}_in")),
        should_clean: true,
    };
    // 有 hash 则写入缓存目录，无则用临时目录
    let output_guard = match cache_hash {
        Some(h) => TempDirGuard {
            path: cache_dir(h)?,
            should_clean: false,
        },
        None => TempDirGuard {
            path: std::env::temp_dir().join(format!("pervius_single_{id}_out")),
            should_clean: true,
        },
    };
    // 写入临时 .class 文件（保持包目录结构）
    let class_file = tmp_input.path.join(class_path);
    if let Some(parent) = class_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&output_guard.path)?;
    std::fs::write(&class_file, bytes)?;
    let mut cmd = process::JavaCommand::new(&vineflower)?;
    cmd.arg("--bytecode-source-mapping=1")
        .arg("--__dump_original_lines__=1")
        .arg(format!("-e={}", jar_path.display()))
        .arg(tmp_input.path.as_os_str())
        .arg(output_guard.path.as_os_str());
    let output = cmd.output().map_err(BridgeError::SpawnFailed)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::ProcessFailed(stderr.into_owned()));
    }
    let base = class_to_base_path(class_path);
    try_read_source(&output_guard.path, base).ok_or(BridgeError::NoOutput)
}
