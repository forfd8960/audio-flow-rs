//! 输入模块

pub mod keyboard;
pub mod clipboard;
pub mod window;

pub use window::{WindowManager, ActiveWindowInfo, InputError, InjectionMethod, TextInputRequest};
pub use keyboard::KeyboardInjector;
pub use clipboard::ClipboardInjector;
