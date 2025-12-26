//! 窗口管理模块
//!
//! 提供主窗口和悬浮窗的管理功能

use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow, WebviewWindowBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 窗口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Main,
    Overlay,
}

/// 窗口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Shown,
    Hidden,
    Minimized,
    Maximized,
}

/// 窗口配置
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// 窗口标题
    pub title: String,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
    /// 是否可调整大小
    pub resizable: bool,
    /// 是否置顶
    pub always_on_top: bool,
    /// 是否透明
    pub transparent: bool,
    /// 是否显示在任务栏
    pub skip_taskbar: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "AudioFlow".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            always_on_top: false,
            transparent: false,
            skip_taskbar: false,
        }
    }
}

/// 悬浮窗配置
impl WindowConfig {
    pub fn overlay() -> Self {
        Self {
            title: "AudioFlow".to_string(),
            width: 400,
            height: 80,
            resizable: false,
            always_on_top: true,
            transparent: true,
            skip_taskbar: true,
        }
    }
}

/// 窗口管理器
#[derive(Debug)]
pub struct WindowManager {
    /// 是否已初始化
    initialized: Arc<AtomicBool>,
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
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 初始化主窗口
    pub fn init_main_window<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        config: Option<WindowConfig>,
    ) -> Result<WebviewWindow<R>, tauri::Error> {
        let config = config.unwrap_or_default();

        let window = WebviewWindowBuilder::new(
            app,
            "main",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title(&config.title)
        .inner_size(config.width as f64, config.height as f64)
        .resizable(config.resizable)
        .always_on_top(config.always_on_top)
        .skip_taskbar(config.skip_taskbar)
        .enable_clipboard_access()
        .build()?;

        Ok(window)
    }

    /// 初始化悬浮窗
    pub fn init_overlay_window<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        config: Option<WindowConfig>,
    ) -> Result<WebviewWindow<R>, tauri::Error> {
        let config = config.unwrap_or_else(WindowConfig::overlay);

        let window = WebviewWindowBuilder::new(
            app,
            "overlay",
            tauri::WebviewUrl::App("overlay.html".into()),
        )
        .title(&config.title)
        .inner_size(config.width as f64, config.height as f64)
        .resizable(config.resizable)
        .always_on_top(config.always_on_top)
        .skip_taskbar(config.skip_taskbar)
        .decorations(false)
        .center()
        .build()?;

        Ok(window)
    }

    /// 初始化所有窗口
    pub fn init_all<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        main_config: Option<WindowConfig>,
        overlay_config: Option<WindowConfig>,
    ) -> Result<(), tauri::Error> {
        self.init_main_window(app, main_config)?;
        self.init_overlay_window(app, overlay_config)?;
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 显示主窗口
    pub fn show_main<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("main") {
            window.show()?;
            window.set_focus()?;
        }
        Ok(())
    }

    /// 隐藏主窗口
    pub fn hide_main<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("main") {
            window.hide()?;
        }
        Ok(())
    }

    /// 显示悬浮窗
    pub fn show_overlay<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("overlay") {
            window.show()?;
            window.set_focus()?;
        }
        Ok(())
    }

    /// 隐藏悬浮窗
    pub fn hide_overlay<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("overlay") {
            window.hide()?;
        }
        Ok(())
    }

    /// 切换悬浮窗显示/隐藏
    pub fn toggle_overlay<R: Runtime>(&self, app: &AppHandle<R>) -> Result<bool, tauri::Error> {
        if let Some(window) = app.get_webview_window("overlay") {
            let is_visible = window.is_visible()?;
            if is_visible {
                self.hide_overlay(app)?;
            } else {
                self.show_overlay(app)?;
            }
            Ok(!is_visible)
        } else {
            self.show_overlay(app)?;
            Ok(true)
        }
    }

    /// 发送数据到主窗口
    pub fn send_to_main<R: Runtime, T: serde::Serialize + Clone>(
        &self,
        app: &AppHandle<R>,
        event: &str,
        data: T,
    ) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("main") {
            window.emit(event, data)?;
        }
        Ok(())
    }

    /// 发送数据到悬浮窗
    pub fn send_to_overlay<R: Runtime, T: serde::Serialize + Clone>(
        &self,
        app: &AppHandle<R>,
        event: &str,
        data: T,
    ) -> Result<(), tauri::Error> {
        if let Some(window) = app.get_webview_window("overlay") {
            window.emit(event, data)?;
        }
        Ok(())
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// 获取窗口状态
    pub fn get_window_state<R: Runtime>(&self, app: &AppHandle<R>, window_type: WindowType) -> WindowState {
        match window_type {
            WindowType::Main => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        return WindowState::Shown;
                    }
                }
            }
            WindowType::Overlay => {
                if let Some(window) = app.get_webview_window("overlay") {
                    if window.is_visible().unwrap_or(false) {
                        return WindowState::Shown;
                    }
                }
            }
        }
        WindowState::Hidden
    }
}

/// 窗口位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPosition {
    Top,
    Bottom,
    Center,
    FollowCursor,
}

impl Default for WindowPosition {
    fn default() -> Self {
        WindowPosition::Center
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.resizable);
    }

    #[test]
    fn test_window_config_overlay() {
        let config = WindowConfig::overlay();
        assert_eq!(config.width, 400);
        assert_eq!(config.height, 80);
        assert!(!config.resizable);
        assert!(config.always_on_top);
        assert!(config.transparent);
        assert!(config.skip_taskbar);
    }

    #[test]
    fn test_window_state_variants() {
        assert_ne!(WindowState::Shown, WindowState::Hidden);
        assert_ne!(WindowState::Minimized, WindowState::Maximized);
    }

    #[test]
    fn test_window_position_variants() {
        assert_ne!(WindowPosition::Top, WindowPosition::Bottom);
        assert_ne!(WindowPosition::Center, WindowPosition::FollowCursor);
    }

    #[test]
    fn test_window_type_variants() {
        assert_ne!(WindowType::Main, WindowType::Overlay);
    }

    #[test]
    fn test_window_manager_create() {
        let manager = WindowManager::new();
        assert!(!manager.is_initialized());
    }
}
