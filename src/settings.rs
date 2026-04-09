//! 用户配置：数据定义 + TOML 持久化
//!
//! 配置文件路径：`{config_dir}/pervius/settings.toml`
//!
//! 所有 section 均标注 `#[serde(default)]`，新增字段不会破坏旧配置文件。
//!
//! @author sky

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 最近打开列表上限
const MAX_RECENT: usize = 10;

/// 顶层配置
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub java: JavaSettings,
    pub recent: Vec<RecentEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            java: JavaSettings::default(),
            recent: Vec::new(),
        }
    }
}

/// 最近打开的文件条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentEntry {
    /// 文件完整路径
    pub path: String,
    /// 文件名（显示用）
    pub name: String,
    /// 打开时间（unix epoch 秒）
    pub timestamp: u64,
}

/// Java 环境配置
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct JavaSettings {
    /// JAVA_HOME 路径（空字符串表示使用系统环境变量）
    pub java_home: String,
}

impl Default for JavaSettings {
    fn default() -> Self {
        Self {
            java_home: String::new(),
        }
    }
}

impl Settings {
    /// 配置目录
    fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("pervius"))
    }

    /// 配置文件路径
    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("settings.toml"))
    }

    /// 从磁盘加载配置，文件不存在或格式错误则返回默认值
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            log::warn!("无法确定配置目录，使用默认配置");
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(settings) => {
                    log::info!("已加载配置：{}", path.display());
                    settings
                }
                Err(e) => {
                    log::warn!("配置解析失败 ({}): {e}，使用默认配置", path.display());
                    Self::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                log::info!("配置文件不存在，使用默认配置");
                Self::default()
            }
            Err(e) => {
                log::warn!("读取配置失败 ({}): {e}，使用默认配置", path.display());
                Self::default()
            }
        }
    }

    /// 保存配置到磁盘
    pub fn save(&self) -> io::Result<()> {
        let Some(dir) = Self::config_dir() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "无法确定配置目录"));
        };
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("settings.toml");
        let content =
            toml::to_string_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        std::fs::write(&path, content)?;
        log::info!("配置已保存：{}", path.display());
        Ok(())
    }

    /// 解析生效的 java 路径，空字符串时回退到 JAVA_HOME 环境变量
    pub fn java_executable(&self) -> Option<PathBuf> {
        let home = if self.java.java_home.is_empty() {
            std::env::var("JAVA_HOME").ok()?
        } else {
            self.java.java_home.clone()
        };
        let path = PathBuf::from(&home);
        if !path.exists() {
            return None;
        }
        let exe = path
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        if exe.exists() {
            Some(exe)
        } else {
            None
        }
    }

    /// 将文件加入最近打开列表头部（去重 + 截断）
    pub fn add_recent(&mut self, path: &Path, name: &str) {
        let path_str = path.to_string_lossy().into_owned();
        self.recent.retain(|e| e.path != path_str);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.recent.insert(
            0,
            RecentEntry {
                path: path_str,
                name: name.to_owned(),
                timestamp,
            },
        );
        self.recent.truncate(MAX_RECENT);
    }

    /// 清空最近打开列表
    pub fn clear_recent(&mut self) {
        self.recent.clear();
    }
}
