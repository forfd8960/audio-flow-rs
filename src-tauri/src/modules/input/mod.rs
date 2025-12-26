//! 输入模块
//!
//! 提供键盘模拟、剪贴板管理和窗口检测功能

pub mod keyboard;
pub mod clipboard;
pub mod window;

pub use keyboard::{KeyboardInjector, KeyboardConfig, special_keys};
pub use clipboard::{ClipboardInjector, ClipboardConfig};
pub use window::{
    WindowManager, ActiveWindowInfo, WindowBounds,
    InputManager, InputConfig,
    InjectionMethod, TextInputRequest
};
pub use crate::error::InputError;
