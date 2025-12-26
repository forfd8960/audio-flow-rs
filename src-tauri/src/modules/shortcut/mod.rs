//! 快捷键模块
//!
//! 提供全局快捷键管理功能

pub mod manager;

pub use manager::{HotkeyManager, HotkeyError, HotkeyState, parse_shortcut};
