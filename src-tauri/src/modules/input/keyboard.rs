//! 键盘模拟模块
//!
//! 使用 enigo 库进行键盘输入模拟

use crate::error::InputError;
use enigo::{Enigo, Keyboard, Key, Settings};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 键盘配置
#[derive(Debug, Clone, Copy)]
pub struct KeyboardConfig {
    /// 每个字符之间的延迟 (毫秒)
    pub char_delay_ms: u64,
    /// 是否启用键盘模拟
    pub enabled: bool,
    /// 模拟速率 (字符/秒)
    pub typing_speed: u16,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            char_delay_ms: 10,
            enabled: true,
            typing_speed: 60,
        }
    }
}

/// 键盘注入器
///
/// 使用 enigo 进行键盘输入模拟，支持文本注入和特殊键处理
#[derive(Debug)]
pub struct KeyboardInjector {
    /// enigo 实例
    enigo: Option<Enigo>,
    /// 配置
    config: KeyboardConfig,
    /// 是否正在运行
    running: AtomicBool,
    /// 最后注入时间
    last_inject: AtomicBool,
}

unsafe impl Send for KeyboardInjector {}
unsafe impl Sync for KeyboardInjector {}

impl Default for KeyboardInjector {
    fn default() -> Self {
        Self::new(KeyboardConfig::default())
    }
}

impl KeyboardInjector {
    /// 创建新的键盘注入器
    pub fn new(config: KeyboardConfig) -> Self {
        let settings = Settings {
            ..Default::default()
        };
        let enigo = match Enigo::new(&settings) {
            Ok(e) => {
                tracing::info!("Keyboard injector initialized successfully");
                Some(e)
            }
            Err(e) => {
                tracing::warn!("Failed to create Enigo instance: {:?}. Keyboard injection disabled. Please grant accessibility permissions.", e);
                tracing::warn!("Go to System Preferences > Privacy & Security > Accessibility to grant permission.");
                None
            }
        };

        Self {
            enigo,
            config,
            running: AtomicBool::new(false),
            last_inject: AtomicBool::new(false),
        }
    }

    /// 检查键盘注入是否可用
    pub fn is_available(&self) -> bool {
        self.enigo.is_some()
    }

    /// 注入文本
    ///
    /// # Arguments
    /// * `text` - 要注入的文本
    ///
    /// # Returns
    /// 注入结果
    pub fn inject(&mut self, text: &str) -> Result<(), InputError> {
        if !self.config.enabled {
            return Ok(());
        }

        self.last_inject.store(true, Ordering::SeqCst);
        let delay_ms = self.config.char_delay_ms;

        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                tracing::warn!("Keyboard injection not available - no accessibility permissions");
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };

        for ch in text.chars() {
            Self::inject_char(enigo, ch)?;
            Self::delay(delay_ms);
        }

        Ok(())
    }

    /// 注入单个字符
    fn inject_char(enigo: &mut Enigo, ch: char) -> Result<(), InputError> {
        match ch {
            // 可打印字符
            c if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' => {
                enigo.text(&c.to_string())
                    .map_err(|_e| InputError::InjectionFailed("Failed to type character".to_string()))?;
            }
            // 换行符
            '\n' | '\r' => {
                enigo.key(Key::Return, enigo::Direction::Click)
                    .map_err(|_e| InputError::InjectionFailed("Failed to press Return".to_string()))?;
            }
            // Tab
            '\t' => {
                enigo.key(Key::Tab, enigo::Direction::Click)
                    .map_err(|_e| InputError::InjectionFailed("Failed to press Tab".to_string()))?;
            }
            // 退格
            '\x08' => {
                enigo.key(Key::Backspace, enigo::Direction::Click)
                    .map_err(|_e| InputError::InjectionFailed("Failed to press Backspace".to_string()))?;
            }
            // 其他控制字符，跳过
            _ => {
                tracing::warn!("Unsupported character: {:?}", ch);
            }
        }

        Ok(())
    }

    /// 注入文本并自动处理特殊字符
    pub fn inject_text(&mut self, text: &str) -> Result<(), InputError> {
        self.inject(text)
    }

    /// 注入单个键
    pub fn inject_key(&mut self, key: Key) -> Result<(), InputError> {
        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };
        enigo.key(key, enigo::Direction::Click)
            .map_err(|_e| InputError::InjectionFailed("Failed to press key".to_string()))?;
        Ok(())
    }

    /// 按下并释放键
    pub fn tap_key(&mut self, key: Key) -> Result<(), InputError> {
        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };
        enigo.key(key, enigo::Direction::Press)
            .map_err(|_e| InputError::InjectionFailed("Failed to press key".to_string()))?;
        enigo.key(key, enigo::Direction::Release)
            .map_err(|_e| InputError::InjectionFailed("Failed to release key".to_string()))?;
        Ok(())
    }

    /// 按下修饰键
    pub fn press_key(&mut self, key: Key) -> Result<(), InputError> {
        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };
        enigo.key(key, enigo::Direction::Press)
            .map_err(|_e| InputError::InjectionFailed("Failed to press key".to_string()))?;
        Ok(())
    }

    /// 释放修饰键
    pub fn release_key(&mut self, key: Key) -> Result<(), InputError> {
        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };
        enigo.key(key, enigo::Direction::Release)
            .map_err(|_e| InputError::InjectionFailed("Failed to release key".to_string()))?;
        Ok(())
    }

    /// 模拟快捷键 (例如 Ctrl+V)
    /// 简化版本：只支持 Ctrl+Key 的常见模式
    pub fn inject_shortcut(&mut self, key: Key, ctrl: bool, alt: bool, shift: bool) -> Result<(), InputError> {
        use enigo::Direction;

        let enigo = match self.enigo.as_mut() {
            Some(e) => e,
            None => {
                return Err(InputError::PermissionDenied("Keyboard injection not available".to_string()));
            }
        };

        // 模拟修饰键按下
        if ctrl {
            enigo.key(Key::Control, Direction::Press)
                .map_err(|_e| InputError::InjectionFailed("Failed to press Control".to_string()))?;
        }
        if alt {
            enigo.key(Key::Alt, Direction::Press)
                .map_err(|_e| InputError::InjectionFailed("Failed to press Alt".to_string()))?;
        }
        if shift {
            enigo.key(Key::Shift, Direction::Press)
                .map_err(|_e| InputError::InjectionFailed("Failed to press Shift".to_string()))?;
        }

        // 按下目标键
        enigo.key(key, Direction::Press)
            .map_err(|_e| InputError::InjectionFailed("Failed to press key".to_string()))?;
        enigo.key(key, Direction::Release)
            .map_err(|_e| InputError::InjectionFailed("Failed to release key".to_string()))?;

        // 释放修饰键 (反向顺序)
        if shift {
            enigo.key(Key::Shift, Direction::Release)
                .map_err(|_e| InputError::InjectionFailed("Failed to release Shift".to_string()))?;
        }
        if alt {
            enigo.key(Key::Alt, Direction::Release)
                .map_err(|_e| InputError::InjectionFailed("Failed to release Alt".to_string()))?;
        }
        if ctrl {
            enigo.key(Key::Control, Direction::Release)
                .map_err(|_e| InputError::InjectionFailed("Failed to release Control".to_string()))?;
        }

        Ok(())
    }

    /// 延迟
    fn delay(delay_ms: u64) {
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    /// 获取配置
    pub fn config(&self) -> KeyboardConfig {
        self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: KeyboardConfig) {
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

    /// 检查是否可以注入
    pub fn can_inject(&self) -> bool {
        self.config.enabled && !self.last_inject.load(Ordering::SeqCst)
    }
}

/// 特殊键映射
pub mod special_keys {
    use enigo::Key;

    /// 将字符串转换为特殊键
    pub fn parse_key(key_str: &str) -> Option<Key> {
        match key_str.to_lowercase().as_str() {
            "enter" | "return" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "space" => Some(Key::Space),
            "backspace" | "bs" => Some(Key::Backspace),
            "delete" | "del" => Some(Key::Delete),
            "up" => Some(Key::UpArrow),
            "down" => Some(Key::DownArrow),
            "left" => Some(Key::LeftArrow),
            "right" => Some(Key::RightArrow),
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" => Some(Key::PageUp),
            "pagedown" => Some(Key::PageDown),
            "escape" | "esc" => Some(Key::Escape),
            "f1" => Some(Key::F1),
            "f2" => Some(Key::F2),
            "f3" => Some(Key::F3),
            "f4" => Some(Key::F4),
            "f5" => Some(Key::F5),
            "f6" => Some(Key::F6),
            "f7" => Some(Key::F7),
            "f8" => Some(Key::F8),
            "f9" => Some(Key::F9),
            "f10" => Some(Key::F10),
            "f11" => Some(Key::F11),
            "f12" => Some(Key::F12),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_config_default() {
        let config = KeyboardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.char_delay_ms, 10);
    }

    #[test]
    fn test_special_keys_parse() {
        assert!(special_keys::parse_key("enter").is_some());
        assert!(special_keys::parse_key("tab").is_some());
        assert!(special_keys::parse_key("escape").is_some());
        assert!(special_keys::parse_key("unknown").is_none());
    }

    #[test]
    #[ignore = "Requires macOS accessibility permissions"]
    fn test_keyboard_injector_create() {
        let injector = KeyboardInjector::new(KeyboardConfig::default());
        assert!(!injector.is_running());
        assert!(injector.can_inject());
    }
}
