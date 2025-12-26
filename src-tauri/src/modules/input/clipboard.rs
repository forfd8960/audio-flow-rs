//! 剪贴板注入模块
//!
//! 使用 Tauri 剪贴板管理器进行文本注入

use crate::error::InputError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Manager;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// 剪贴板配置
#[derive(Debug, Clone, Copy)]
pub struct ClipboardConfig {
    /// 粘贴后等待时间 (毫秒)
    pub paste_wait_ms: u64,
    /// 是否在注入后恢复剪贴板内容
    pub restore_clipboard: bool,
    /// 是否启用剪贴板注入
    pub enabled: bool,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            paste_wait_ms: 100,
            restore_clipboard: true,
            enabled: true,
        }
    }
}

/// 剪贴板注入器
///
/// 使用 Tauri 剪贴板管理器进行文本注入
#[derive(Debug)]
pub struct ClipboardInjector {
    /// 配置
    config: ClipboardConfig,
    /// 是否正在运行
    running: AtomicBool,
    /// 上次保存的剪贴板内容
    saved_content: std::sync::Mutex<Option<String>>,
}

impl Default for ClipboardInjector {
    fn default() -> Self {
        Self::new(ClipboardConfig::default())
    }
}

impl ClipboardInjector {
    /// 创建新的剪贴板注入器
    pub fn new(config: ClipboardConfig) -> Self {
        Self {
            config,
            running: AtomicBool::new(false),
            saved_content: std::sync::Mutex::new(None),
        }
    }

    /// 注入文本到剪贴板并执行粘贴
    ///
    /// # Arguments
    /// * `app` - Tauri 应用句柄
    /// * `text` - 要注入的文本
    ///
    /// # Returns
    /// 注入结果
    pub async fn inject(&self, app: &tauri::AppHandle, text: &str) -> Result<(), InputError> {
        if !self.config.enabled {
            return Ok(());
        }

        // 保存当前剪贴板内容
        if self.config.restore_clipboard {
            self.save_clipboard(app)?;
        }

        // 设置新文本
        app.clipboard()
            .write_text(text)
            .map_err(|e| InputError::InjectionFailed(e.to_string()))?;

        // 执行粘贴 (Ctrl+V 或 Cmd+V)
        self.paste()?;

        // 等待
        if self.config.paste_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.config.paste_wait_ms)).await;
        }

        // 恢复剪贴板
        if self.config.restore_clipboard {
            self.restore_clipboard(app)?;
        }

        Ok(())
    }

    /// 执行粘贴操作
    fn paste(&self) -> Result<(), InputError> {
        // 使用键盘模拟执行 Ctrl+V / Cmd+V
        // 在实际应用中，这里应该使用全局快捷键模拟
        tracing::info!("Paste operation triggered");
        Ok(())
    }

    /// 保存当前剪贴板内容
    fn save_clipboard(&self, app: &tauri::AppHandle) -> Result<(), InputError> {
        if let Ok(content) = app.clipboard().read_text() {
            let mut saved = self.saved_content.lock().unwrap();
            *saved = Some(content);
        }
        Ok(())
    }

    /// 恢复剪贴板内容
    fn restore_clipboard(&self, app: &tauri::AppHandle) -> Result<(), InputError> {
        let saved = self.saved_content.lock().unwrap();
        if let Some(content) = &*saved {
            app.clipboard()
                .write_text(content)
                .map_err(|_e| InputError::ClipboardRestoreFailed)?;
        }
        Ok(())
    }

    /// 直接写入剪贴板
    pub fn write(&self, app: &tauri::AppHandle, text: &str) -> Result<(), InputError> {
        app.clipboard()
            .write_text(text)
            .map_err(|_e| InputError::ClipboardFailed)?;
        Ok(())
    }

    /// 从剪贴板读取
    pub fn read(&self, app: &tauri::AppHandle) -> Result<Option<String>, InputError> {
        match app.clipboard().read_text() {
            Ok(text) => Ok(Some(text)),
            Err(_) => Err(InputError::ClipboardFailed),
        }
    }

    /// 获取配置
    pub fn config(&self) -> ClipboardConfig {
        self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: ClipboardConfig) {
        self.config = config;
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 启动注入器
    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// 停止注入器
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_config_default() {
        let config = ClipboardConfig::default();
        assert!(config.enabled);
        assert!(config.restore_clipboard);
        assert_eq!(config.paste_wait_ms, 100);
    }

    #[test]
    fn test_clipboard_injector_create() {
        let injector = ClipboardInjector::new(ClipboardConfig::default());
        assert!(!injector.is_running());
    }
}
