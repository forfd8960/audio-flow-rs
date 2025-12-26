//! 系统托盘模块
//!
//! 提供托盘图标创建、菜单管理和事件处理

use tauri::{
    AppHandle, Manager, Runtime, Emitter,
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// 录音状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Listening,
    Recording,
    Paused,
    Transcribing,
}

/// 托盘状态管理
#[derive(Debug)]
pub struct TrayState {
    is_recording: AtomicBool,
    recording_state: AtomicBool,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            is_recording: AtomicBool::new(false),
            recording_state: AtomicBool::new(false),
        }
    }
}

impl TrayState {
    pub fn set_recording(&self, recording: bool) {
        self.is_recording.store(recording, Ordering::SeqCst);
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

/// 托盘图标管理器
#[derive(Debug)]
pub struct TrayManager {
    state: TrayState,
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayManager {
    pub fn new() -> Self {
        Self {
            state: TrayState::default(),
        }
    }

    /// 创建托盘图标和菜单
    pub fn create_tray<R: Runtime>(&self, app: &AppHandle<R>) -> Result<tauri::tray::TrayIcon<R>, tauri::Error> {
        let toggle_recording = MenuItem::with_id(app, "toggle_recording", "开始录音", true, None::<&str>)?;
        let show_window = MenuItem::with_id(app, "show_window", "显示窗口", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

        let menu = Menu::with_items(app, &[&toggle_recording, &show_window, &quit])?;

        // 加载托盘图标
        let icon = load_tray_icon()?;

        let tray = TrayIconBuilder::new()
            .icon(icon)
            .menu(&menu)
            .tooltip("AudioFlow - 语音转文字")
            .on_menu_event(move |app, event| {
                match event.id.as_ref() {
                    "toggle_recording" => {
                        // 切换录音状态
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("toggle-recording", ());
                        }
                    }
                    "show_window" => {
                        // 显示主窗口
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            })
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                    // 左键点击显示窗口
                    if let Some(app) = tray.app_handle().get_webview_window("main") {
                        let _ = app.show();
                        let _ = app.set_focus();
                    }
                }
            })
            .build(app)?;

        Ok(tray)
    }

    /// 更新托盘图标状态
    pub fn update_state(&self, recording: bool) {
        self.state.set_recording(recording);
    }

    /// 获取状态
    pub fn state(&self) -> &TrayState {
        &self.state
    }
}

/// 加载托盘图标
fn load_tray_icon() -> Result<Image<'static>, tauri::Error> {
    // 尝试从资源加载图标
    // 如果没有自定义图标，使用内置图标

    // 创建简单的 32x32 RGBA 图标
    // 这是一个简单的麦克风形状的像素数组
    let icon_data = include_bytes!("../../../icons/tray_icon.png");

    if icon_data.is_empty() {
        // 如果没有图标文件，创建默认图标
        create_default_icon()
    } else {
        Image::from_bytes(icon_data)
    }
}

/// 创建默认托盘图标 (简单的圆形)
fn create_default_icon() -> Result<Image<'static>, tauri::Error> {
    // 创建一个简单的 32x32 像素图标
    // 像素格式: RGBA
    const SIZE: usize = 32;
    const HALF: usize = SIZE / 2;

    let mut pixels = Vec::with_capacity(SIZE * SIZE * 4);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as i32 - HALF as i32;
            let dy = y as i32 - HALF as i32;
            let dist = ((dx * dx + dy * dy) as f32).sqrt();

            let (r, g, b, a) = if dist < HALF as f32 - 2.0 {
                // 外圆 - 深蓝色
                (30, 100, 180, 255)
            } else if dist < HALF as f32 {
                // 边框
                (20, 80, 160, 255)
            } else {
                // 透明
                (0, 0, 0, 0)
            };

            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }

    Image::from_bytes(&pixels)
}

/// 托盘菜单 ID
pub trait TrayMenuIds {
    const TOGGLE_RECORDING: &'static str = "toggle_recording";
    const SHOW_WINDOW: &'static str = "show_window";
    const QUIT: &'static str = "quit";
    const SETTINGS: &'static str = "settings";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_state_default() {
        let state = TrayState::default();
        assert!(!state.is_recording());
    }

    #[test]
    fn test_tray_state_recording() {
        let state = TrayState::default();
        state.set_recording(true);
        assert!(state.is_recording());
        state.set_recording(false);
        assert!(!state.is_recording());
    }

    #[test]
    fn test_recording_state_variants() {
        assert_ne!(RecordingState::Idle, RecordingState::Recording);
        assert_ne!(RecordingState::Listening, RecordingState::Transcribing);
    }
}
