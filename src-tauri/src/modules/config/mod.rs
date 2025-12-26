//! 配置模块

pub mod manager;
pub mod secure_storage;

pub use manager::{ConfigManager, UserConfig, ApiConfig, AudioSettings, InputSettings, UiSettings, HotkeySettings};
pub use secure_storage::{SecureStorage, SecureStorageError, ApiKeyStorage, ElevenLabsKeyStorage};
