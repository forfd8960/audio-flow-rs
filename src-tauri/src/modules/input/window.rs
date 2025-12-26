//! 活跃窗口检测模块

use thiserror::Error;
use std::fmt;

/// 活跃窗口信息
#[derive(Debug, Clone)]
pub struct ActiveWindowInfo {
    pub process_id: u32,
    pub app_name: String,
    pub window_title: String,
    pub is_editable: bool,
}

/// 输入错误
#[derive(Debug, Error)]
pub enum InputError {
    #[error("No active window found")]
    NoActiveWindow,
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Injection failed: {0}")]
    InjectionFailed(String),
}

/// 文本注入方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InjectionMethod {
    #[default]
    Keyboard,
    Clipboard,
    Auto,
}

/// 文本注入请求
#[derive(Debug, Default)]
pub struct TextInputRequest {
    pub text: String,
    pub method: InjectionMethod,
    pub priority: u8,
}

/// 窗口管理器
#[derive(Debug, Default)]
pub struct WindowManager;

impl WindowManager {
    pub fn get_active_window(&self) -> Result<ActiveWindowInfo, InputError> {
        Ok(ActiveWindowInfo {
            process_id: 0,
            app_name: "Unknown".to_string(),
            window_title: "Unknown".to_string(),
            is_editable: false,
        })
    }
}

impl fmt::Display for ActiveWindowInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {} (PID: {})", self.app_name, self.window_title, self.process_id)
    }
}
