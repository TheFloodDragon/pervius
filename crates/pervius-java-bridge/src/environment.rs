//! 外部 Java 依赖管理：配置同步、目录解析、Maven 下载与校验
//!
//! @author sky

use crate::decompiler;
use crate::error::BridgeError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 默认 Vineflower 版本
pub const DEFAULT_VINEFLOWER_VERSION: &str = "1.12.0";
/// 默认 Kotlin 版本
pub const DEFAULT_KOTLIN_VERSION: &str = "2.0.21";

const MAVEN_BASE: &str = "https://repo.huaweicloud.com/repository/maven";

/// 外部依赖配置（由 UI 设置同步到 bridge 层）。
#[derive(Clone, Debug)]
pub struct EnvironmentConfig {
    /// Vineflower 版本
    pub vineflower_version: String,
    /// Vineflower 存储目录；None 表示默认目录。
    pub vineflower_dir: Option<PathBuf>,
    /// Kotlin 版本
    pub kotlin_version: String,
    /// Kotlin 依赖存储目录；None 表示默认目录。
    pub kotlin_dependencies_dir: Option<PathBuf>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            vineflower_version: DEFAULT_VINEFLOWER_VERSION.to_string(),
            vineflower_dir: None,
            kotlin_version: DEFAULT_KOTLIN_VERSION.to_string(),
            kotlin_dependencies_dir: None,
        }
    }
}

/// 已解析的 Kotlin 依赖路径。
#[derive(Clone, Debug)]
pub struct KotlinDependencies {
    /// kotlin-stdlib JAR
    pub stdlib: PathBuf,
    /// kotlin-compiler-embeddable JAR
    pub compiler_embeddable: PathBuf,
}

#[derive(Clone, Copy)]
struct MavenJar<'a> {
    group_path: &'a str,
    artifact_id: &'a str,
    version: &'a str,
}

impl MavenJar<'_> {
    fn file_name(self) -> String {
        format!("{}-{}.jar", self.artifact_id, self.version)
    }

    fn jar_url(self) -> String {
        format!(
            "{}/{}/{}/{}/{}",
            MAVEN_BASE,
            self.group_path,
            self.artifact_id,
            self.version,
            self.file_name()
        )
    }

    fn sha256_url(self) -> String {
        format!("{}.sha256", self.jar_url())
    }
}

static ENVIRONMENT_CONFIG: Mutex<EnvironmentConfig> = Mutex::new(EnvironmentConfig {
    vineflower_version: String::new(),
    vineflower_dir: None,
    kotlin_version: String::new(),
    kotlin_dependencies_dir: None,
});

/// 设置外部依赖配置。
pub fn set_environment_config(config: EnvironmentConfig) {
    let mut lock = ENVIRONMENT_CONFIG.lock().unwrap_or_else(|p| p.into_inner());
    *lock = normalize_config(config);
}

/// 获取当前外部依赖配置。
pub fn environment_config() -> EnvironmentConfig {
    let config = ENVIRONMENT_CONFIG
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    normalize_config(config)
}

/// 获取当前 Vineflower 版本。
pub fn vineflower_version() -> String {
    environment_config().vineflower_version
}

/// 获取当前 Kotlin 版本。
pub fn kotlin_version() -> String {
    environment_config().kotlin_version
}

/// 当前生效 Vineflower 存储目录。
pub fn current_vineflower_dir() -> Result<PathBuf, BridgeError> {
    Ok(
        environment_config()
            .vineflower_dir
            .unwrap_or(default_environment_root()?.join("vineflower")),
    )
}

/// 当前生效 Kotlin 依赖存储目录。
pub fn current_kotlin_dependencies_dir() -> Result<PathBuf, BridgeError> {
    Ok(
        environment_config()
            .kotlin_dependencies_dir
            .unwrap_or(default_environment_root()?.join("kotlin")),
    )
}

/// 定位并按需下载 Vineflower。
pub fn ensure_vineflower() -> Result<PathBuf, BridgeError> {
    let version = vineflower_version();
    let artifact = MavenJar {
        group_path: "org/vineflower",
        artifact_id: "vineflower",
        version: &version,
    };
    resolve_maven_jar(&current_vineflower_dir()?, artifact)
}

/// 定位并按需下载 Kotlin 编译依赖。
pub fn ensure_kotlin_dependencies() -> Result<KotlinDependencies, BridgeError> {
    let version = kotlin_version();
    let dir = current_kotlin_dependencies_dir()?;
    Ok(KotlinDependencies {
        stdlib: resolve_maven_jar(
            &dir,
            MavenJar {
                group_path: "org/jetbrains/kotlin",
                artifact_id: "kotlin-stdlib",
                version: &version,
            },
        )?,
        compiler_embeddable: resolve_maven_jar(
            &dir,
            MavenJar {
                group_path: "org/jetbrains/kotlin",
                artifact_id: "kotlin-compiler-embeddable",
                version: &version,
            },
        )?,
    })
}

fn normalize_config(mut config: EnvironmentConfig) -> EnvironmentConfig {
    if config.vineflower_version.trim().is_empty() {
        config.vineflower_version = DEFAULT_VINEFLOWER_VERSION.to_string();
    } else {
        config.vineflower_version = config.vineflower_version.trim().to_string();
    }
    if config.kotlin_version.trim().is_empty() {
        config.kotlin_version = DEFAULT_KOTLIN_VERSION.to_string();
    } else {
        config.kotlin_version = config.kotlin_version.trim().to_string();
    }
    config.vineflower_dir = normalize_dir(config.vineflower_dir);
    config.kotlin_dependencies_dir = normalize_dir(config.kotlin_dependencies_dir);
    config
}

fn normalize_dir(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|p| !p.as_os_str().is_empty())
}

fn default_environment_root() -> Result<PathBuf, BridgeError> {
    Ok(decompiler::current_cache_root()?.join("tools"))
}

fn resolve_maven_jar(dir: &Path, artifact: MavenJar<'_>) -> Result<PathBuf, BridgeError> {
    let file_name = artifact.file_name();
    if let Some(path) = jar_next_to_exe(&file_name) {
        return Ok(path);
    }
    ensure_verified_download(dir, &file_name, &artifact.jar_url(), &artifact.sha256_url())
}

fn jar_next_to_exe(file_name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join(file_name);
    if is_non_empty_file(&path) {
        Some(path)
    } else {
        None
    }
}

fn ensure_verified_download(
    dir: &Path,
    file_name: &str,
    jar_url: &str,
    checksum_url: &str,
) -> Result<PathBuf, BridgeError> {
    std::fs::create_dir_all(dir)?;
    let expected = fetch_sha256(checksum_url)?;
    let target = dir.join(file_name);
    if verify_existing_file(&target, &expected)? {
        return Ok(target);
    }
    log::info!("Downloading {file_name} from {jar_url}");
    let tmp = dir.join(format!(".{file_name}.download"));
    cleanup_file(&tmp);
    download_to_file(jar_url, &tmp)?;
    verify_sha256_file(&tmp, &expected, file_name)?;
    if target.exists() {
        cleanup_file(&target);
    }
    std::fs::rename(&tmp, &target)?;
    Ok(target)
}

fn verify_existing_file(path: &Path, expected: &str) -> Result<bool, BridgeError> {
    if !is_non_empty_file(path) {
        return Ok(false);
    }
    match sha256_file(path) {
        Ok(actual) if actual.eq_ignore_ascii_case(expected) => Ok(true),
        Ok(actual) => {
            log::warn!(
                "Checksum mismatch for {}, expected {expected}, got {actual}; re-downloading",
                path.display()
            );
            cleanup_file(path);
            Ok(false)
        }
        Err(error) => {
            log::warn!(
                "Failed to verify existing download {}: {error}; re-downloading",
                path.display()
            );
            cleanup_file(path);
            Ok(false)
        }
    }
}

fn verify_sha256_file(path: &Path, expected: &str, file_name: &str) -> Result<(), BridgeError> {
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    cleanup_file(path);
    Err(BridgeError::Download(format!(
        "checksum mismatch for {file_name}: expected {expected}, got {actual}"
    )))
}

fn fetch_sha256(url: &str) -> Result<String, BridgeError> {
    let text = download_text(url)?;
    let checksum = text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if checksum.len() != 64 || !checksum.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(BridgeError::Download(format!(
            "invalid sha256 response from {url}: {text}"
        )));
    }
    Ok(checksum)
}

fn download_text(url: &str) -> Result<String, BridgeError> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| BridgeError::Download(format!("failed to download {url}: {e}")))?;
    response
        .into_string()
        .map_err(|e| BridgeError::Download(format!("failed to read {url}: {e}")))
}

fn download_to_file(url: &str, path: &Path) -> Result<(), BridgeError> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| BridgeError::Download(format!("failed to download {url}: {e}")))?;
    let mut reader = response.into_reader();
    let mut file = File::create(path)?;
    std::io::copy(&mut reader, &mut file)?;
    file.flush()?;
    if !is_non_empty_file(path) {
        cleanup_file(path);
        return Err(BridgeError::Download(format!(
            "downloaded file is empty: {}",
            path.display()
        )));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, BridgeError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(bytes_to_hex(hasher.finalize().as_slice()))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn cleanup_file(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

fn is_non_empty_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}
