//! Tauri 命令
//!
//! 定义前后端通信的命令，包括音频控制、网络连接、输入注入等

use crate::error::{AppError, AudioError, NetworkError, InputError, ConfigError};
use crate::modules::audio::{AudioCapturer, VoiceActivityDetector, VadLevel};
use crate::modules::network::scribe_client::ScribeClient;
use crate::modules::input::{InputManager, InputConfig, InjectionMethod, ActiveWindowInfo};
use crate::modules::shortcut::{HotkeyManager, HotkeyState};
use crate::modules::config::{ConfigManager, UserConfig};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, AppHandle, Manager, State};
use tauri::async_runtime::Mutex as TauriMutex;

// ============ 通用类型定义 ============

/// 音频设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub device_id: String,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rate: u32,
}

/// 录音状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Listening,
    Recording,
    Paused,
    Transcribing,
}

/// 录音状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub state: RecordingState,
    pub duration_ms: u64,
    pub volume_level: f32,
    pub is_speech: bool,
}

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub state: String,
    pub attempt: u32,
}

/// 活跃窗口信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub process_id: u32,
    pub app_name: String,
    pub window_title: String,
    pub is_editable: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 输入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfigDto {
    pub default_method: String,
    pub keyboard_enabled: bool,
    pub keyboard_char_delay_ms: u64,
    pub clipboard_enabled: bool,
    pub clipboard_paste_wait_ms: u64,
    pub restore_clipboard: bool,
    pub typing_speed: u16,
}

/// 快捷键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub shortcut: String,
    pub is_registered: bool,
    pub state: String,
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
    pub model_id: String,
    pub language_code: String,
}

/// 转换结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub is_final: bool,
    pub confidence: f32,
    pub timestamp_ms: u64,
}

// ============ 应用状态管理命令 ============

/// 获取应用版本
#[command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取应用名称
#[command]
pub fn get_app_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

// ============ 音频命令 ============

/// 获取可用的音频输入设备
#[command]
pub async fn get_audio_devices(app: AppHandle) -> Result<Vec<AudioDeviceInfo>, String> {
    let devices = AudioCapturer::available_devices()
        .map_err(|e| format!("Failed to list devices: {}", e))?;

    let default_device = AudioCapturer::default_device()
        .ok()
        .map(|d| d.name);

    Ok(devices
        .into_iter()
        .map(|d| AudioDeviceInfo {
            name: d.name.clone(),
            device_id: d.id,
            is_default: Some(&d.name) == default_device.as_ref(),
            channels: d.channels,
            sample_rate: d.sample_rates.first().copied().unwrap_or(44100),
        })
        .collect())
}

/// 启动录音
#[command]
pub async fn start_listen(app: AppHandle) -> Result<RecordingStatus, String> {
    let state = app.state::<AppState>();

    if state.is_recording() {
        return Err("Already recording".to_string());
    }

    state.set_recording(true);

    Ok(RecordingStatus {
        state: RecordingState::Listening,
        duration_ms: 0,
        volume_level: 0.0,
        is_speech: false,
    })
}

/// 停止录音
#[command]
pub async fn stop_listen(app: AppHandle) -> Result<RecordingStatus, String> {
    let state = app.state::<AppState>();

    if !state.is_recording() {
        return Err("Not recording".to_string());
    }

    state.set_recording(false);

    Ok(RecordingStatus {
        state: RecordingState::Idle,
        duration_ms: 0,
        volume_level: 0.0,
        is_speech: false,
    })
}

/// 获取当前录音状态
#[command]
pub async fn get_recording_status(app: AppHandle) -> Result<RecordingStatus, String> {
    let state = app.state::<AppState>();

    Ok(RecordingStatus {
        state: if state.is_recording() {
            RecordingState::Listening
        } else {
            RecordingState::Idle
        },
        duration_ms: 0,
        volume_level: 0.0,
        is_speech: false,
    })
}

// ============ 网络命令 ============

/// 连接到语音转写服务
#[command]
pub async fn connect_scribe(
    app: AppHandle,
    api_key: String,
    model_id: String,
    language_code: String,
) -> Result<ConnectionStatus, String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let mut guard = client.lock().await;

    guard.set_api_key(api_key);
    guard.update_config(crate::modules::network::scribe_client::ScribeConfig {
        model_id,
        language_code,
        ..Default::default()
    });

    match guard.connect().await {
        Ok(_) => {
            let state = app.state::<AppState>();
            state.set_connected(true);
            Ok(ConnectionStatus {
                is_connected: true,
                state: "Connected".to_string(),
                attempt: 0,
            })
        }
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

/// 断开连接
#[command]
pub async fn disconnect_scribe(app: AppHandle) -> Result<ConnectionStatus, String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let mut guard = client.lock().await;

    guard.disconnect().await;
    let state = app.state::<AppState>();
    state.set_connected(false);

    Ok(ConnectionStatus {
        is_connected: false,
        state: "Disconnected".to_string(),
        attempt: 0,
    })
}

/// 获取连接状态
#[command]
pub async fn get_connection_status(app: AppHandle) -> Result<ConnectionStatus, String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let guard = client.lock().await;
    let state = guard.connection_state();

    Ok(ConnectionStatus {
        is_connected: state.is_connected(),
        state: state.to_string(),
        attempt: 0,
    })
}

/// 发送音频数据
#[command]
pub async fn send_audio_chunk(app: AppHandle, audio_data: Vec<f32>) -> Result<(), String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let mut guard = client.lock().await;

    guard.send_audio(&audio_data).await
        .map_err(|e| format!("Failed to send audio: {}", e))
}

/// 接收转写结果
#[command]
pub async fn receive_transcription(app: AppHandle) -> Result<Option<TranscriptionResult>, String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let mut guard = client.lock().await;

    match guard.receive_response().await {
        Ok(Some(response)) => {
            let text = response.text;
            let is_final = response.is_final;

            // 清理文本
            let text = text
                .replace("【SPEECH_CHANGE】", "")
                .replace("【SILENCE】", "")
                .trim()
                .to_string();

            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(TranscriptionResult {
                    text,
                    is_final,
                    confidence: response.confidence as f32,
                    timestamp_ms: 0,
                }))
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Failed to receive transcription: {}", e)),
    }
}

// ============ 输入注入命令 ============

/// 获取当前活跃窗口信息
#[command]
pub fn get_active_window(app: AppHandle) -> Result<WindowInfo, String> {
    let input_manager = app.state::<TauriMutex<InputManager>>();
    let guard = input_manager.blocking_lock();

    match guard.get_active_window() {
        Ok(window) => {
            let bounds = window.bounds.as_ref();
            Ok(WindowInfo {
                process_id: window.process_id,
                app_name: window.app_name,
                window_title: window.window_title,
                is_editable: window.is_editable,
                x: bounds.map(|b| b.x).unwrap_or(0),
                y: bounds.map(|b| b.y).unwrap_or(0),
                width: bounds.map(|b| b.width).unwrap_or(0),
                height: bounds.map(|b| b.height).unwrap_or(0),
            })
        }
        Err(e) => Err(format!("Failed to get active window: {}", e)),
    }
}

/// 注入文本到当前窗口
#[command]
pub async fn inject_text(app: AppHandle, text: String) -> Result<(), String> {
    let input_manager = app.state::<TauriMutex<InputManager>>();
    let mut guard = input_manager.lock().await;

    guard.inject(&text, &app)
        .await
        .map_err(|e| format!("Failed to inject text: {}", e))
}

/// 设置输入配置
#[command]
pub fn update_input_config(app: AppHandle, config: InputConfigDto) -> Result<(), String> {
    let input_manager = app.state::<TauriMutex<InputManager>>();
    let mut guard = input_manager.blocking_lock();

    let method = match config.default_method.as_str() {
        "keyboard" => InjectionMethod::Keyboard,
        "clipboard" => InjectionMethod::Clipboard,
        _ => InjectionMethod::Auto,
    };

    let input_config = InputConfig {
        default_method: method,
        keyboard_enabled: config.keyboard_enabled,
        keyboard_char_delay_ms: config.keyboard_char_delay_ms,
        clipboard_enabled: config.clipboard_enabled,
        clipboard_paste_wait_ms: config.clipboard_paste_wait_ms,
        restore_clipboard: config.restore_clipboard,
        typing_speed: config.typing_speed,
    };

    guard.update_config(input_config);
    Ok(())
}

/// 获取当前输入配置
#[command]
pub fn get_input_config(app: AppHandle) -> Result<InputConfigDto, String> {
    let input_manager = app.state::<TauriMutex<InputManager>>();
    let guard = input_manager.blocking_lock();
    let config = guard.config();

    let method_str = match config.default_method {
        InjectionMethod::Keyboard => "keyboard",
        InjectionMethod::Clipboard => "clipboard",
        InjectionMethod::Auto => "auto",
    };

    Ok(InputConfigDto {
        default_method: method_str.to_string(),
        keyboard_enabled: config.keyboard_enabled,
        keyboard_char_delay_ms: config.keyboard_char_delay_ms,
        clipboard_enabled: config.clipboard_enabled,
        clipboard_paste_wait_ms: config.clipboard_paste_wait_ms,
        restore_clipboard: config.restore_clipboard,
        typing_speed: config.typing_speed,
    })
}

// ============ 快捷键命令 ============

/// 获取默认快捷键
#[command]
pub fn get_default_shortcut(app: AppHandle) -> Result<ShortcutConfig, String> {
    let manager = app.state::<HotkeyManager>();

    Ok(ShortcutConfig {
        shortcut: manager.default_shortcut().to_string(),
        is_registered: manager.is_registered(&app, &manager.default_shortcut()),
        state: format!("{:?}", manager.current_state()),
    })
}

/// 注册快捷键
#[command]
pub fn register_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    let manager = app.state::<HotkeyManager>();
    manager.register(&app, &shortcut)
        .map_err(|e| format!("Failed to register shortcut: {}", e))
}

/// 注销快捷键
#[command]
pub fn unregister_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    let manager = app.state::<HotkeyManager>();
    manager.unregister(&app, &shortcut)
        .map_err(|e| format!("Failed to unregister shortcut: {}", e))
}

/// 获取所有已注册的快捷键
#[command]
pub fn get_registered_shortcuts(app: AppHandle) -> Result<Vec<String>, String> {
    let manager = app.state::<HotkeyManager>();
    Ok(manager.registered_shortcuts())
}

// ============ 配置命令 ============

/// 加载配置
#[command]
pub fn load_config(app: AppHandle) -> Result<serde_json::Value, String> {
    let config_manager = app.state::<ConfigManager>();
    let config = config_manager.load()
        .map_err(|e| format!("Failed to load config: {}", e))?;
    serde_json::to_value(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

/// 保存配置
#[command]
pub fn save_config(app: AppHandle, config: serde_json::Value) -> Result<(), String> {
    let config_manager = app.state::<ConfigManager>();
    // Convert serde_json::Value to UserConfig
    let user_config: UserConfig = serde_json::from_value(config)
        .map_err(|e| format!("Invalid config format: {}", e))?;
    config_manager.save(&user_config)
        .map_err(|e| format!("Failed to save config: {}", e))
}

/// 获取 API 配置
#[command]
pub fn get_api_config(app: AppHandle) -> Result<ApiConfig, String> {
    let config_manager = app.state::<ConfigManager>();
    let config = config_manager.load()
        .map_err(|e| format!("Failed to load config: {}", e))?;

    Ok(ApiConfig {
        api_key: config.api.elevenlabs_api_key.unwrap_or_default(),
        model_id: config.api.model_id,
        language_code: config.api.language_code,
    })
}

/// 设置 API 密钥
#[command]
pub fn set_api_key(app: AppHandle, api_key: String) -> Result<(), String> {
    let config_manager = app.state::<ConfigManager>();
    config_manager.update(|config| {
        config.api.elevenlabs_api_key = Some(api_key);
    })
    .map_err(|e| format!("Failed to save API key: {}", e))
}

// ============ VAD 命令 ============

/// 设置 VAD 级别
#[command]
pub async fn set_vad_level(app: AppHandle, level: String) -> Result<(), String> {
    let vad_level = match level.as_str() {
        "low" | "aggressive" => VadLevel::Aggressive,
        "medium" | "balanced" => VadLevel::Balanced,
        "high" | "relaxed" => VadLevel::Relaxed,
        _ => VadLevel::Balanced,
    };

    let client = app.state::<TauriMutex<ScribeClient>>();
    let mut guard = client.lock().await;
    guard.set_vad_level(vad_level);
    Ok(())
}

/// 获取 VAD 级别
#[command]
pub async fn get_vad_level(app: AppHandle) -> Result<String, String> {
    let client = app.state::<TauriMutex<ScribeClient>>();
    let guard = client.lock().await;

    let level = guard.vad_level();
    let level_str = match level {
        VadLevel::Aggressive => "aggressive",
        VadLevel::Balanced => "balanced",
        VadLevel::Relaxed => "relaxed",
    };

    Ok(level_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_status_serialization() {
        let status = RecordingStatus {
            state: RecordingState::Recording,
            duration_ms: 1000,
            volume_level: 0.5,
            is_speech: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Recording"));
    }

    #[test]
    fn test_window_info_serialization() {
        let info = WindowInfo {
            process_id: 1234,
            app_name: "Test App".to_string(),
            window_title: "Test Window".to_string(),
            is_editable: true,
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Test App"));
    }

    #[test]
    fn test_input_config_dto_default() {
        let dto = InputConfigDto {
            default_method: "auto".to_string(),
            keyboard_enabled: true,
            keyboard_char_delay_ms: 10,
            clipboard_enabled: true,
            clipboard_paste_wait_ms: 100,
            restore_clipboard: true,
            typing_speed: 60,
        };
        assert_eq!(dto.default_method, "auto");
    }
}
