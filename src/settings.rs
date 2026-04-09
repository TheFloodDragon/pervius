//! 用户配置：数据定义 + 业务逻辑
//!
//! TOML 持久化由 [`egui_window_settings::SettingsFile`] trait 提供，
//! 所有 section 均标注 `#[serde(default)]`，新增字段不会破坏旧配置文件。
//!
//! @author sky

use egui_window_settings::SettingsFile;
use serde::{Deserialize, Serialize};
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

impl SettingsFile for Settings {
    fn app_name() -> &'static str {
        "pervius"
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
