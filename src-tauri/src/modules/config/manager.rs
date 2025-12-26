//! 配置管理器

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use arc_swap::ArcSwap;

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadFailed(String),
    #[error("Failed to save configuration: {0}")]
    SaveFailed(String),
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    pub elevenlabs_api_key: Option<String>,
    pub language_code: String,
    pub model_id: String,
}

/// 音频设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioSettings {
    pub input_device: Option<String>,
    pub sample_rate: u32,
    pub noise_suppression: bool,
    pub auto_gain: bool,
}

/// 输入设置
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum InjectionMethod {
    Keyboard,
    Clipboard,
    #[default]
    Auto,
}

/// UI 主题
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// UI 位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum OverlayPosition {
    Top,
    Bottom,
    #[default]
    Center,
    FollowCursor,
}

/// 输入设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputSettings {
    pub injection_method: InjectionMethod,
    pub auto_hide_overlay: bool,
    pub keyboard_typing_speed: u8,
    pub clipboard_restore: bool,
}

/// UI 设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiSettings {
    pub overlay_opacity: f64,
    pub overlay_position: OverlayPosition,
    pub theme: Theme,
}

/// 快捷键设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HotkeySettings {
    pub listen_key: String,
    pub listen_modifiers: Vec<String>,
}

/// 用户配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub api: ApiConfig,
    pub audio: AudioSettings,
    pub input: InputSettings,
    pub hotkeys: HotkeySettings,
    pub ui: UiSettings,
}

/// 配置管理器
#[derive(Debug)]
pub struct ConfigManager {
    config: ArcSwap<UserConfig>,
    config_path: PathBuf,
}

impl Default for ConfigManager {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("audio-flow");
        Self::new(config_dir)
    }
}

impl ConfigManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let config_path = config_dir.join("config.toml");
        let config = ArcSwap::new(Arc::new(UserConfig::default()));
        Self { config, config_path }
    }

    pub fn load(&self) -> Result<UserConfig, ConfigError> {
        if !self.config_path.exists() {
            return Ok(UserConfig::default());
        }
        let content = std::fs::read_to_string(&self.config_path)
            .map_err(|e| ConfigError::LoadFailed(e.to_string()))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::LoadFailed(e.to_string()))
    }

    pub fn save(&self, config: &UserConfig) -> Result<(), ConfigError> {
        let content = toml::to_string(config)
            .map_err(|e| ConfigError::SaveFailed(e.to_string()))?;
        std::fs::write(&self.config_path, content)
            .map_err(|e| ConfigError::SaveFailed(e.to_string()))?;
        self.config.store(Arc::new(config.clone()));
        Ok(())
    }

    pub fn current(&self) -> Arc<UserConfig> {
        (*self.config.load()).clone()
    }

    pub fn update<F>(&self, f: F) -> Result<(), ConfigError>
    where F: FnOnce(&mut UserConfig) {
        let mut config = (*self.current()).clone();
        f(&mut config);
        self.save(&config)
    }
}
