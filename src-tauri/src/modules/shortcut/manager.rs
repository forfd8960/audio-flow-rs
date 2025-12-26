//! 全局快捷键管理器
//!
//! 使用 tauri-plugin-global-shortcut 实现全局快捷键注册
//!
//! 注意：快捷键回调通过 Tauri 事件系统在前端处理

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use thiserror::Error;

/// 快捷键错误
#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("Failed to register shortcut: {0}")]
    RegistrationFailed(String),
    #[error("Shortcut not found: {0}")]
    NotFound(String),
    #[error("Invalid shortcut format: {0}")]
    InvalidFormat(String),
    #[error("Shortcut already registered: {0}")]
    AlreadyRegistered(String),
}

/// 快捷键状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotkeyState {
    #[default]
    Idle,
    Listening,
    Transcribing,
}

/// 快捷键管理器
///
/// 使用 tauri-plugin-global-shortcut 实现全局快捷键管理
#[derive(Debug)]
pub struct HotkeyManager {
    /// 已注册的快捷键集合
    registered_shortcuts: Arc<parking_lot::Mutex<HashSet<String>>>,
    /// 当前快捷键状态
    hotkey_state: Arc<parking_lot::Mutex<HotkeyState>>,
    /// 默认快捷键
    default_shortcut: Arc<parking_lot::Mutex<String>>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 初始化标志
    initialized: Arc<AtomicBool>,
}

impl HotkeyManager {
    /// 创建新的快捷键管理器
    pub fn new() -> Self {
        Self {
            registered_shortcuts: Arc::new(parking_lot::Mutex::new(HashSet::new())),
            hotkey_state: Arc::new(parking_lot::Mutex::new(HotkeyState::Idle)),
            default_shortcut: Arc::new(parking_lot::Mutex::new("CmdOrCtrl+Shift+S".to_string())),
            running: Arc::new(AtomicBool::new(false)),
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 初始化并注册默认快捷键
    pub fn init(&self, app: &AppHandle) -> Result<(), HotkeyError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.register_default(app)?;
        self.running.store(true, Ordering::SeqCst);
        self.initialized.store(true, Ordering::SeqCst);
        tracing::info!("Hotkey manager initialized with default shortcut: {}", self.default_shortcut());
        Ok(())
    }

    /// 注册默认快捷键
    pub fn register_default(&self, app: &AppHandle) -> Result<(), HotkeyError> {
        self.register(app, &self.default_shortcut())
    }

    /// 注册快捷键 (使用 Tauri API)
    pub fn register(&self, app: &AppHandle, shortcut: &str) -> Result<(), HotkeyError> {
        let normalized = Self::normalize_shortcut(shortcut);

        // 检查是否已注册
        {
            let registered = self.registered_shortcuts.lock();
            if registered.contains(&normalized) {
                return Err(HotkeyError::AlreadyRegistered(shortcut.to_string()));
            }
        }

        // 使用 Tauri global shortcut plugin 注册
        match app.global_shortcut().register(shortcut) {
            Ok(_) => {
                let mut registered = self.registered_shortcuts.lock();
                registered.insert(normalized.clone());
                tracing::info!("Registered shortcut: {}", shortcut);
                Ok(())
            }
            Err(e) => Err(HotkeyError::RegistrationFailed(e.to_string())),
        }
    }

    /// 注销快捷键
    pub fn unregister(&self, app: &AppHandle, shortcut: &str) -> Result<(), HotkeyError> {
        match app.global_shortcut().unregister(shortcut) {
            Ok(_) => {
                let normalized = Self::normalize_shortcut(shortcut);
                let mut registered = self.registered_shortcuts.lock();
                registered.remove(&normalized);
                tracing::info!("Unregistered shortcut: {}", shortcut);
                Ok(())
            }
            Err(e) => Err(HotkeyError::NotFound(e.to_string())),
        }
    }

    /// 注销所有快捷键
    pub fn unregister_all(&self, app: &AppHandle) -> Result<(), HotkeyError> {
        match app.global_shortcut().unregister_all() {
            Ok(_) => {
                let mut registered = self.registered_shortcuts.lock();
                registered.clear();
                tracing::info!("Unregistered all shortcuts");
                Ok(())
            }
            Err(e) => Err(HotkeyError::RegistrationFailed(e.to_string())),
        }
    }

    /// 检查快捷键是否被按下
    pub fn is_pressed(&self) -> bool {
        let state = self.hotkey_state.lock();
        matches!(*state, HotkeyState::Transcribing)
    }

    /// 重置按下状态
    pub fn reset_pressed(&self) {
        let mut state = self.hotkey_state.lock();
        *state = HotkeyState::Idle;
    }

    /// 获取当前状态
    pub fn current_state(&self) -> HotkeyState {
        *self.hotkey_state.lock()
    }

    /// 设置当前状态
    pub fn set_state(&self, state: HotkeyState) {
        let mut guard = self.hotkey_state.lock();
        *guard = state;
    }

    /// 获取默认快捷键
    pub fn default_shortcut(&self) -> String {
        self.default_shortcut.lock().clone()
    }

    /// 设置默认快捷键
    pub fn set_default_shortcut(&self, shortcut: &str) {
        let mut guard = self.default_shortcut.lock();
        *guard = Self::normalize_shortcut(shortcut);
    }

    /// 检查快捷键是否已注册
    pub fn is_registered(&self, app: &AppHandle, shortcut: &str) -> bool {
        app.global_shortcut().is_registered(shortcut)
    }

    /// 获取所有已注册的快捷键
    pub fn registered_shortcuts(&self) -> Vec<String> {
        let registered = self.registered_shortcuts.lock();
        registered.iter().cloned().collect()
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 启动快捷键管理器
    pub fn start(&self, app: &AppHandle) -> Result<(), HotkeyError> {
        if !self.is_running() {
            self.init(app)?;
        }
        Ok(())
    }

    /// 停止快捷键管理器
    pub fn stop(&self, app: &AppHandle) -> Result<(), HotkeyError> {
        if self.is_running() {
            self.unregister_all(app)?;
            self.running.store(false, Ordering::SeqCst);
            self.initialized.store(false, Ordering::SeqCst);
        }
        Ok(())
    }

    /// 标准化快捷键格式
    fn normalize_shortcut(shortcut: &str) -> String {
        let parts: Vec<&str> = shortcut.split('+').collect();
        parts
            .into_iter()
            .map(|s| {
                let lower = s.to_lowercase();
                // 标准化修饰键名称
                match lower.as_str() {
                    "cmd" | "command" => "CmdOrCtrl".to_string(),
                    "ctrl" | "control" => "Control".to_string(),
                    "alt" | "option" => "Alt".to_string(),
                    "shift" => "Shift".to_string(),
                    "super" => "Super".to_string(),
                    _ => lower,
                }
            })
            .collect::<Vec<String>>()
            .join("+")
    }
}

/// 解析快捷键字符串
pub fn parse_shortcut(shortcut: &str) -> Result<(Vec<String>, String), HotkeyError> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.len() < 2 {
        return Err(HotkeyError::InvalidFormat(shortcut.to_string()));
    }
    let key = parts.last().unwrap().to_string();
    let modifiers: Vec<String> = parts[..parts.len() - 1].iter().map(|s| s.to_string()).collect();
    Ok((modifiers, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_shortcut_cmd_ctrl() {
        assert_eq!(
            HotkeyManager::normalize_shortcut("cmd+s"),
            "CmdOrCtrl+s"
        );
        assert_eq!(
            HotkeyManager::normalize_shortcut("Ctrl+Shift+S"),
            "Control+Shift+s"
        );
    }

    #[test]
    fn test_parse_shortcut() {
        let (modifiers, key) = parse_shortcut("CmdOrCtrl+Shift+S").unwrap();
        assert_eq!(modifiers, vec!["CmdOrCtrl", "Shift"]);
        assert_eq!(key, "S");
    }

    #[test]
    fn test_parse_shortcut_invalid() {
        assert!(parse_shortcut("S").is_err());
    }
}
