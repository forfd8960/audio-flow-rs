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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InjectionMethod {
    Keyboard,
    Clipboard,
    #[default]
    Auto,
}

/// UI 主题
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

/// UI 位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_manager_create() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        let config = manager.current();
        // Default struct values: sample_rate = 0 (empty struct default)
        assert_eq!(config.audio.sample_rate, 0);
    }

    #[test]
    fn test_config_manager_load_default() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());
        let config = manager.load().unwrap();
        // Default struct values: sample_rate = 0
        assert_eq!(config.audio.sample_rate, 0);
        assert!(!config.audio.noise_suppression); // bool defaults to false
    }

    #[test]
    fn test_config_manager_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let mut manager = ConfigManager::new(temp_dir.path().to_path_buf());

        // Update config
        let new_config = UserConfig {
            api: ApiConfig {
                elevenlabs_api_key: Some("test-key".to_string()),
                language_code: "en".to_string(),
                model_id: "scribe_v1".to_string(),
            },
            audio: AudioSettings {
                sample_rate: 48000,
                noise_suppression: false,
                ..Default::default()
            },
            ..Default::default()
        };

        manager.save(&new_config).unwrap();

        // Create new manager and load
        let manager2 = ConfigManager::new(temp_dir.path().to_path_buf());
        let loaded = manager2.load().unwrap();

        assert_eq!(loaded.api.elevenlabs_api_key, Some("test-key".to_string()));
        assert_eq!(loaded.api.language_code, "en");
        assert_eq!(loaded.audio.sample_rate, 48000);
        assert!(!loaded.audio.noise_suppression);
    }

    #[test]
    fn test_config_manager_update() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigManager::new(temp_dir.path().to_path_buf());

        manager.update(|config| {
            config.audio.sample_rate = 44100;
            config.ui.theme = Theme::Dark;
        }).unwrap();

        let updated = manager.current();
        assert_eq!(updated.audio.sample_rate, 44100);
        assert_eq!(updated.ui.theme, Theme::Dark);
    }

    #[test]
    fn test_config_manager_load_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let manager = ConfigManager::new(temp_dir.path().join("nonexistent.toml"));
        let config = manager.load().unwrap();
        // Returns default config when file doesn't exist
        assert_eq!(config.audio.sample_rate, 0);
    }

    #[test]
    fn test_config_toml_serialization() {
        let config = UserConfig {
            api: ApiConfig {
                elevenlabs_api_key: Some("test-key".to_string()),
                language_code: "zh".to_string(),
                model_id: "scribe_v1".to_string(),
            },
            audio: AudioSettings {
                sample_rate: 16000,
                noise_suppression: true,
                auto_gain: false,
                input_device: Some("Microphone".to_string()),
            },
            input: InputSettings {
                injection_method: InjectionMethod::Clipboard,
                auto_hide_overlay: true,
                keyboard_typing_speed: 5,
                clipboard_restore: true,
            },
            hotkeys: HotkeySettings {
                listen_key: "space".to_string(),
                listen_modifiers: vec!["cmd".to_string(), "shift".to_string()],
            },
            ui: UiSettings {
                overlay_opacity: 0.8,
                overlay_position: OverlayPosition::Top,
                theme: Theme::Light,
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: UserConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.api.elevenlabs_api_key, config.api.elevenlabs_api_key);
        assert_eq!(parsed.api.language_code, config.api.language_code);
        assert_eq!(parsed.audio.sample_rate, config.audio.sample_rate);
        assert_eq!(parsed.input.injection_method, config.input.injection_method);
        assert_eq!(parsed.hotkeys.listen_key, config.hotkeys.listen_key);
    }

    #[test]
    fn test_user_config_default() {
        let config = UserConfig::default();
        // Default struct values
        assert_eq!(config.audio.sample_rate, 0);
        assert!(!config.audio.noise_suppression);
        assert!(!config.audio.auto_gain);
        assert_eq!(config.input.injection_method, InjectionMethod::Auto);
        assert!(!config.input.clipboard_restore);
        assert_eq!(config.ui.theme, Theme::System);
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert!(config.elevenlabs_api_key.is_none());
        assert_eq!(config.language_code, ""); // empty string default
        assert_eq!(config.model_id, "");
    }

    #[test]
    fn test_audio_settings_default() {
        let settings = AudioSettings::default();
        assert_eq!(settings.sample_rate, 0);
        assert!(!settings.noise_suppression);
        assert!(!settings.auto_gain);
        assert!(settings.input_device.is_none());
    }

    #[test]
    fn test_input_settings_default() {
        let settings = InputSettings::default();
        assert_eq!(settings.injection_method, InjectionMethod::Auto);
        assert!(!settings.auto_hide_overlay); // bool defaults to false
        assert_eq!(settings.keyboard_typing_speed, 0);
        assert!(!settings.clipboard_restore);
    }

    #[test]
    fn test_ui_settings_default() {
        let settings = UiSettings::default();
        assert_eq!(settings.overlay_opacity, 0.0); // f64 defaults to 0.0
        assert_eq!(settings.overlay_position, OverlayPosition::Center);
        assert_eq!(settings.theme, Theme::System);
    }

    #[test]
    fn test_hotkey_settings_default() {
        let settings = HotkeySettings::default();
        assert_eq!(settings.listen_key, ""); // String defaults to empty
        assert!(settings.listen_modifiers.is_empty());
    }

    #[test]
    fn test_injection_method_variants() {
        assert_ne!(InjectionMethod::Keyboard, InjectionMethod::Clipboard);
        assert_ne!(InjectionMethod::Clipboard, InjectionMethod::Auto);
    }

    #[test]
    fn test_theme_variants() {
        assert_ne!(Theme::Light, Theme::Dark);
        assert_ne!(Theme::Dark, Theme::System);
    }

    #[test]
    fn test_overlay_position_variants() {
        assert_ne!(OverlayPosition::Top, OverlayPosition::Bottom);
        assert_ne!(OverlayPosition::Bottom, OverlayPosition::Center);
        assert_ne!(OverlayPosition::Center, OverlayPosition::FollowCursor);
    }

    #[test]
    fn test_config_error_load_failed() {
        let error = ConfigError::LoadFailed("file not found".to_string());
        assert!(error.to_string().contains("Failed to load configuration"));
    }

    #[test]
    fn test_config_error_save_failed() {
        let error = ConfigError::SaveFailed("permission denied".to_string());
        assert!(error.to_string().contains("Failed to save configuration"));
    }
}
