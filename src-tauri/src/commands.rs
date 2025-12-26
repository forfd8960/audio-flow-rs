//! Tauri 命令
//!
//! 定义前后端通信的命令

use tauri::command;

#[command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to AudioFlow.", name)
}

#[command]
pub async fn start_listen(_app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn stop_listen(_app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}
