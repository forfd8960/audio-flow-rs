//! AudioFlow 核心库
//!
//! Tauri 后端入口点

#![allow(unused)]

mod commands;
mod error;
mod events;
mod state;
mod modules;

use anyhow::Result;
use commands::*;
use modules::config::ConfigManager;
use modules::input::InputManager;
use modules::network::scribe_client::ScribeClient;
use modules::shortcut::HotkeyManager;
use tauri::Manager;

pub use commands::*;
pub use error::{AppError as AudioFlowError, ErrorCode};
pub use state::AppState;

const APP_DIR: &str = "audio-flow";

/// 初始化应用
pub fn init_app() -> Result<()> {
    use tracing_subscriber::fmt;
    fmt::init();
    Ok(())
}

/// 运行应用
#[allow(dead_code)]
pub fn run() -> Result<()> {
    init_app()?;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(APP_DIR);

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    let runtime_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("audio-flow.log".to_string()),
                    },
                ))
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            // 应用信息
            get_app_version,
            get_app_name,
            // 音频
            get_audio_devices,
            start_listen,
            stop_listen,
            get_recording_status,
            // 网络
            connect_scribe,
            disconnect_scribe,
            get_connection_status,
            send_audio_chunk,
            receive_transcription,
            // 输入
            get_active_window,
            inject_text,
            update_input_config,
            get_input_config,
            // 快捷键
            get_default_shortcut,
            register_shortcut,
            unregister_shortcut,
            get_registered_shortcuts,
            // 配置
            load_config,
            save_config,
            get_api_config,
            set_api_key,
            // VAD
            set_vad_level,
            get_vad_level,
        ])
        .setup(move |app| {
            // 管理工作状态
            app.manage(runtime_state);

            // 管理配置
            let config_manager = ConfigManager::new(config_dir.clone());
            app.manage(config_manager);

            // 管理 Scribe 客户端
            let scribe_client = tauri::async_runtime::Mutex::new(ScribeClient::default());
            app.manage(scribe_client);

            // 管理输入管理器
            let input_manager = tauri::async_runtime::Mutex::new(InputManager::new());
            app.manage(input_manager);

            // 初始化快捷键管理器
            let shortcut_manager = HotkeyManager::new();
            app.manage(shortcut_manager);

            // 注册全局快捷键 (在 setup 中使用窗口状态)
            // 注意：在 setup 中我们不能直接获取 &mut HotkeyManager
            // 但可以在命令中延迟初始化

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
