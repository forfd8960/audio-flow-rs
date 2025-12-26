//! 键盘模拟模块

use super::{InputError, TextInputRequest};

/// 键盘配置
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardConfig {
    pub char_delay_ms: u64,
}

/// 键盘注入器
#[derive(Debug, Default)]
pub struct KeyboardInjector;

impl KeyboardInjector {
    pub fn new(_config: KeyboardConfig) -> Self {
        Self
    }

    pub fn inject(&self, _text: &str) -> Result<(), InputError> {
        Ok(())
    }

    pub fn handle_request(&self, _request: &TextInputRequest) -> Result<(), InputError> {
        Ok(())
    }
}
