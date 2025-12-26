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
use modules::config::ConfigManager;
use tauri::Manager;

pub use commands::{greet, start_listen, stop_listen};
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
            greet,
            start_listen,
            stop_listen,
        ])
        .setup(|app| {
            app.manage(runtime_state);
            let config_manager = ConfigManager::new(config_dir);
            app.manage(config_manager);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
