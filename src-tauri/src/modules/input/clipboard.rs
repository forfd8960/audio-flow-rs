//! 剪贴板注入模块

use super::{InputError, TextInputRequest};
use tauri::Manager;

/// 剪贴板配置
#[derive(Debug, Clone, Copy, Default)]
pub struct ClipboardConfig {
    pub paste_wait_ms: u64,
    pub restore_clipboard: bool,
}

/// 剪贴板注入器
#[derive(Debug, Default)]
pub struct ClipboardInjector;

impl ClipboardInjector {
    pub fn new(_config: ClipboardConfig) -> Self {
        Self
    }

    pub async fn inject(&self, _app: &tauri::AppHandle, _text: &str) -> Result<(), InputError> {
        Ok(())
    }

    pub async fn handle_request(
        &self,
        _app: &tauri::AppHandle,
        _request: &TextInputRequest,
    ) -> Result<(), InputError> {
        Ok(())
    }
}
