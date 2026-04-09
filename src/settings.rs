//! 用户配置：数据定义 + 业务逻辑
//!
//! TOML 持久化由 [`egui_window_settings::SettingsFile`] trait 提供，
//! 所有 section 均标注 `#[serde(default)]`，新增字段不会破坏旧配置文件。
//!
//! @author sky

use egui_keybind::KeyBind;
use egui_window_settings::SettingsFile;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 最近打开列表上限
const MAX_RECENT: usize = 10;

/// 界面语言
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh")]
    Zh,
}

impl Default for Language {
    fn default() -> Self {
        Self::En
    }
}

impl Language {
    pub const ALL: &[Self] = &[Self::En, Self::Zh];

    /// 返回 rust-i18n 使用的 locale code
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
        }
    }

    /// 返回语言显示名称
    pub fn label(self) -> String {
        match self {
            Self::En => t!("lang.en").to_string(),
            Self::Zh => t!("lang.zh").to_string(),
        }
    }
}

/// 顶层配置
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub java: JavaSettings,
    pub keymap: KeymapSettings,
    pub recent: Vec<RecentEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            java: JavaSettings::default(),
            keymap: KeymapSettings::default(),
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

/// 快捷键配置
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KeymapSettings {
    pub toggle_explorer: KeyBind,
    pub open_jar: KeyBind,
    pub find: KeyBind,
    pub find_in_files: KeyBind,
    pub save: KeyBind,
    pub close_tab: KeyBind,
    pub close_all_tabs: KeyBind,
    pub export_decompiled: KeyBind,
    pub cycle_view: KeyBind,
    pub open_settings: KeyBind,
}

impl Default for KeymapSettings {
    fn default() -> Self {
        use crate::ui::keybindings;
        Self {
            toggle_explorer: keybindings::DEFAULT_TOGGLE_EXPLORER,
            open_jar: keybindings::DEFAULT_OPEN_JAR,
            find: keybindings::DEFAULT_FIND,
            find_in_files: keybindings::DEFAULT_FIND_IN_FILES,
            save: keybindings::DEFAULT_SAVE,
            close_tab: keybindings::DEFAULT_CLOSE_TAB,
            close_all_tabs: keybindings::DEFAULT_CLOSE_ALL_TABS,
            export_decompiled: keybindings::DEFAULT_EXPORT_DECOMPILED,
            cycle_view: keybindings::DEFAULT_CYCLE_VIEW,
            open_settings: keybindings::DEFAULT_OPEN_SETTINGS,
        }
    }
}

impl Settings {
    /// 仅读取语言配置（用于启动时在 UI 初始化前设置 locale）
    pub fn load_for_locale() -> Self {
        Self::load()
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
