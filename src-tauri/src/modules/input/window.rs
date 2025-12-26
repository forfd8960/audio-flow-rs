//! 活跃窗口检测模块
//!
//! 使用 active-win-pos-rs 获取当前活跃窗口信息

use crate::error::InputError;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex as TokioMutex;

/// 活跃窗口信息
#[derive(Debug, Clone)]
pub struct ActiveWindowInfo {
    /// 进程 ID
    pub process_id: u32,
    /// 应用程序名称
    pub app_name: String,
    /// 窗口标题
    pub window_title: String,
    /// 是否可编辑 (检测是否为文本输入框)
    pub is_editable: bool,
    /// 窗口位置和大小
    pub bounds: Option<WindowBounds>,
}

impl Default for ActiveWindowInfo {
    fn default() -> Self {
        Self {
            process_id: 0,
            app_name: "Unknown".to_string(),
            window_title: "Unknown".to_string(),
            is_editable: false,
            bounds: None,
        }
    }
}

impl fmt::Display for ActiveWindowInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {} (PID: {})", self.app_name, self.window_title, self.process_id)
    }
}

/// 窗口边界
#[derive(Debug, Clone, Default)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
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
#[derive(Debug, Clone)]
pub struct TextInputRequest {
    /// 要注入的文本
    pub text: String,
    /// 注入方法
    pub method: InjectionMethod,
    /// 优先级 (0-255)
    pub priority: u8,
}

impl Default for TextInputRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            method: InjectionMethod::Auto,
            priority: 128,
        }
    }
}

/// 窗口管理器
///
/// 使用 active-win-pos-rs 获取活跃窗口信息
#[derive(Debug)]
pub struct WindowManager {
    /// 是否正在运行
    running: AtomicBool,
    /// 上次检测的窗口
    last_window: std::sync::Mutex<ActiveWindowInfo>,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager {
    /// 创建新的窗口管理器
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            last_window: std::sync::Mutex::new(ActiveWindowInfo::default()),
        }
    }

    /// 获取活跃窗口信息
    ///
    /// # Returns
    /// 活跃窗口信息，如果无法获取则返回默认值
    pub fn get_active_window(&self) -> Result<ActiveWindowInfo, InputError> {
        // 使用 active-win-pos-rs 获取窗口信息
        // 注意：这可能需要辅助功能权限
        match active_win_pos_rs::get_active_window() {
            Ok(window) => {
                // 先检测可编辑性（需要借用window）
                let is_editable = self.detect_editable(&window);

                let info = ActiveWindowInfo {
                    process_id: window.process_id as u32,
                    app_name: window.app_name,
                    window_title: window.title,
                    is_editable,
                    bounds: Some(WindowBounds {
                        x: window.position.x as i32,
                        y: window.position.y as i32,
                        width: window.position.width as u32,
                        height: window.position.height as u32,
                    }),
                };

                // 保存最后检测的窗口
                let mut last = self.last_window.lock().unwrap();
                *last = info.clone();

                Ok(info)
            }
            Err(_) => {
                // 没有活跃窗口或获取失败，返回默认值
                Ok(ActiveWindowInfo::default())
            }
        }
    }

    /// 检测窗口是否可编辑
    fn detect_editable(&self, window: &active_win_pos_rs::ActiveWindow) -> bool {
        // 基于窗口标题和类名检测
        let title = window.title.to_lowercase();
        let app_name = window.app_name.to_lowercase();

        // 可编辑窗口的常见名称模式
        let editable_patterns = [
            "text", "input", "edit", "textarea", "compose",
            "search", "chat", "message", "comment", "document",
            "notepad", "editor", "terminal", "console",
        ];

        // 检查窗口标题
        for pattern in &editable_patterns {
            if title.contains(pattern) {
                return true;
            }
        }

        // 检查应用程序名称
        for pattern in &editable_patterns {
            if app_name.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 检查窗口是否仍然活跃
    pub fn is_window_still_active(&self) -> bool {
        match self.get_active_window() {
            Ok(current) => {
                let last = self.last_window.lock().unwrap();
                current.process_id == last.process_id && !current.window_title.is_empty()
            }
            Err(_) => false,
        }
    }

    /// 获取最后检测的窗口
    pub fn last_window(&self) -> ActiveWindowInfo {
        self.last_window.lock().unwrap().clone()
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 启动窗口管理器
    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// 停止窗口管理器
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// 文本注入器管理器
///
/// 协调键盘和剪贴板注入
#[derive(Debug)]
pub struct InputManager {
    /// 窗口管理器
    window_manager: WindowManager,
    /// 键盘注入器
    keyboard: std::sync::Mutex<super::KeyboardInjector>,
    /// 剪贴板注入器 (使用 tokio::sync::Mutex 支持 async)
    clipboard: TokioMutex<super::ClipboardInjector>,
    /// 配置
    config: std::sync::Mutex<InputConfig>,
    /// 是否正在运行
    running: AtomicBool,
}

unsafe impl Send for InputManager {}
unsafe impl Sync for InputManager {}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    /// 创建新的输入管理器
    pub fn new() -> Self {
        Self {
            window_manager: WindowManager::new(),
            keyboard: std::sync::Mutex::new(super::KeyboardInjector::default()),
            clipboard: TokioMutex::new(super::ClipboardInjector::default()),
            config: std::sync::Mutex::new(InputConfig::default()),
            running: AtomicBool::new(false),
        }
    }

    /// 注入文本
    ///
    /// # Arguments
    /// * `text` - 要注入的文本
    /// * `app` - Tauri 应用句柄
    ///
    /// # Returns
    /// 注入结果
    pub async fn inject(&self, text: &str, app: &tauri::AppHandle) -> Result<(), InputError> {
        let window = self.window_manager.get_active_window()?;

        // 确定注入方法（在持有锁的block内）
        let method = {
            let config = self.config.lock().unwrap();
            match config.default_method {
                InjectionMethod::Auto => {
                    // 基于窗口类型选择方法
                    if window.is_editable {
                        InjectionMethod::Clipboard // 剪贴板更快
                    } else {
                        InjectionMethod::Keyboard
                    }
                }
                _ => config.default_method,
            }
        }; // config 在这里被释放

        match method {
            InjectionMethod::Keyboard => {
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.inject(text)?;
            }
            InjectionMethod::Clipboard => {
                let mut clipboard = self.clipboard.lock().await;
                clipboard.inject(app, text).await?;
            }
            InjectionMethod::Auto => {
                // Auto should have been resolved above, but just in case
                let mut keyboard = self.keyboard.lock().unwrap();
                keyboard.inject(text)?;
            }
        }

        Ok(())
    }

    /// 获取活跃窗口
    pub fn get_active_window(&self) -> Result<ActiveWindowInfo, InputError> {
        self.window_manager.get_active_window()
    }

    /// 获取配置
    pub fn config(&self) -> InputConfig {
        self.config.lock().unwrap().clone()
    }

    /// 更新配置
    pub fn update_config(&self, config: InputConfig) {
        let mut cfg = self.config.lock().unwrap();
        *cfg = config;

        // 更新子组件配置
        let mut keyboard = self.keyboard.lock().unwrap();
        keyboard.update_config(super::KeyboardConfig {
            char_delay_ms: cfg.keyboard_char_delay_ms,
            enabled: cfg.keyboard_enabled,
            typing_speed: cfg.typing_speed,
        });

        // 使用 blocking_lock 因为这是同步函数
        let mut clipboard = self.clipboard.blocking_lock();
        clipboard.update_config(super::ClipboardConfig {
            paste_wait_ms: cfg.clipboard_paste_wait_ms,
            restore_clipboard: cfg.restore_clipboard,
            enabled: cfg.clipboard_enabled,
        });
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 启动输入管理器
    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        self.window_manager.start();
    }

    /// 停止输入管理器
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.window_manager.stop();
    }
}

/// 输入配置
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// 默认注入方法
    pub default_method: InjectionMethod,
    /// 键盘启用
    pub keyboard_enabled: bool,
    /// 键盘字符延迟 (毫秒)
    pub keyboard_char_delay_ms: u64,
    /// 剪贴板启用
    pub clipboard_enabled: bool,
    /// 剪贴板粘贴等待 (毫秒)
    pub clipboard_paste_wait_ms: u64,
    /// 是否恢复剪贴板
    pub restore_clipboard: bool,
    /// 打字速度 (字符/秒)
    pub typing_speed: u16,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            default_method: InjectionMethod::Auto,
            keyboard_enabled: true,
            keyboard_char_delay_ms: 10,
            clipboard_enabled: true,
            clipboard_paste_wait_ms: 100,
            restore_clipboard: true,
            typing_speed: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_window_info_default() {
        let info = ActiveWindowInfo::default();
        assert_eq!(info.process_id, 0);
        assert_eq!(info.app_name, "Unknown");
    }

    #[test]
    fn test_text_input_request_default() {
        let request = TextInputRequest::default();
        assert!(request.text.is_empty());
        assert_eq!(request.method, InjectionMethod::Auto);
    }

    #[test]
    fn test_input_config_default() {
        let config = InputConfig::default();
        assert!(config.keyboard_enabled);
        assert!(config.clipboard_enabled);
        assert_eq!(config.default_method, InjectionMethod::Auto);
    }

    #[test]
    fn test_window_manager_create() {
        let manager = WindowManager::new();
        assert!(!manager.is_running());
    }

    #[test]
    #[ignore = "Requires macOS accessibility permissions for keyboard"]
    fn test_input_manager_create() {
        let manager = InputManager::new();
        assert!(!manager.is_running());
    }
}
