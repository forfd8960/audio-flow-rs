//! 事件分发模块
//!
//! 提供前后端事件通信功能

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 前端事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FrontendEvent {
    /// 录音状态变化
    RecordingStateChanged { is_recording: bool, state: String },
    /// 连接状态变化
    ConnectionStateChanged { is_connected: bool, state: String },
    /// 转写结果
    TranscriptionResult { text: String, is_final: bool },
    /// 音量级别
    VolumeLevel { level: f32, is_speech: bool },
    /// 错误通知
    Error { message: String, code: i32 },
    /// 快捷键按下
    HotkeyPressed { shortcut: String },
    /// 配置已更新
    ConfigUpdated,
}

/// 事件目标
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTarget {
    /// 主窗口
    Main,
    /// 悬浮窗
    Overlay,
    /// 所有窗口
    All,
}

impl Default for EventTarget {
    fn default() -> Self {
        EventTarget::Main
    }
}

/// 事件分发器配置
#[derive(Debug, Clone)]
pub struct EventDispatcherConfig {
    /// 是否启用事件分发
    pub enabled: bool,
    /// 事件缓冲队列大小
    pub buffer_size: usize,
    /// 是否打印事件日志
    pub log_events: bool,
}

impl Default for EventDispatcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 100,
            log_events: cfg!(debug_assertions),
        }
    }
}

/// 事件分发器
///
/// 异步事件分发，支持事件缓冲和批量处理
#[derive(Debug)]
pub struct EventDispatcher {
    /// 应用句柄
    app: Option<tauri::AppHandle>,
    /// 是否启用
    enabled: Arc<AtomicBool>,
    /// 配置
    config: Arc<EventDispatcherConfig>,
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher {
    /// 创建新的事件分发器
    pub fn new() -> Self {
        Self {
            app: None,
            enabled: Arc::new(AtomicBool::new(true)),
            config: Arc::new(EventDispatcherConfig::default()),
        }
    }

    /// 设置应用句柄
    pub fn set_app(&mut self, app: &tauri::AppHandle) {
        self.app = Some(app.clone());
    }

    /// 发送事件到前端
    pub fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }

        if self.config.log_events {
            tracing::debug!("Emitting event: {}", event);
        }

        if let Some(ref app) = self.app {
            if let Err(e) = app.emit(event, payload.clone()) {
                tracing::error!("Failed to emit event {}: {}", event, e);
            }
        }
    }

    /// 发送事件到特定窗口
    pub fn emit_to<S: Serialize + Clone>(&self, target: EventTarget, event: &str, payload: S) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }

        if self.config.log_events {
            tracing::debug!("Emitting event to {:?}: {}", target, event);
        }

        if let Some(ref app) = self.app {
            let result = match target {
                EventTarget::Main => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.emit(event, payload.clone())
                    } else {
                        app.emit(event, payload.clone())
                    }
                }
                EventTarget::Overlay => {
                    if let Some(window) = app.get_webview_window("overlay") {
                        window.emit(event, payload.clone())
                    } else {
                        app.emit(event, payload.clone())
                    }
                }
                EventTarget::All => app.emit(event, payload.clone()),
            };
            if let Err(e) = result {
                tracing::error!("Failed to emit event {} to {:?}: {}", event, target, e);
            }
        }
    }

    /// 发送录音状态变化事件
    pub fn emit_recording_state(&self, is_recording: bool, state: &str) {
        let payload = FrontendEvent::RecordingStateChanged {
            is_recording,
            state: state.to_string(),
        };
        self.emit("recording-state-changed", payload);
    }

    /// 发送连接状态变化事件
    pub fn emit_connection_state(&self, is_connected: bool, state: &str) {
        let payload = FrontendEvent::ConnectionStateChanged {
            is_connected,
            state: state.to_string(),
        };
        self.emit("connection-state-changed", payload);
    }

    /// 发送转写结果事件
    pub fn emit_transcription(&self, text: &str, is_final: bool) {
        let payload = FrontendEvent::TranscriptionResult {
            text: text.to_string(),
            is_final,
        };
        self.emit("transcription-result", payload);
    }

    /// 发送音量级别事件
    pub fn emit_volume_level(&self, level: f32, is_speech: bool) {
        let payload = FrontendEvent::VolumeLevel { level, is_speech };
        self.emit("volume-level", payload);
    }

    /// 发送错误事件
    pub fn emit_error(&self, message: &str, code: i32) {
        let payload = FrontendEvent::Error {
            message: message.to_string(),
            code,
        };
        self.emit("error", payload);
    }

    /// 启用/禁用事件分发
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

/// 事件监听器
///
/// 用于监听前端发送的事件
#[derive(Debug)]
pub struct EventListener {
    /// 接收事件的 channel
    receiver: mpsc::Receiver<(String, serde_json::Value)>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
}

impl EventListener {
    /// 创建新的事件监听器
    pub fn new(buffer_size: usize) -> (Self, mpsc::Sender<(String, serde_json::Value)>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let listener = Self {
            receiver: rx,
            running: Arc::new(AtomicBool::new(true)),
        };
        (listener, tx)
    }

    /// 接收下一个事件
    pub async fn recv(&mut self) -> Option<(String, serde_json::Value)> {
        self.receiver.recv().await
    }

    /// 停止监听
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_frontend_event_serialization() {
        let event = FrontendEvent::RecordingStateChanged {
            is_recording: true,
            state: "Recording".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RecordingStateChanged"));
        assert!(json.contains("is_recording"));
    }

    #[test]
    fn test_frontend_event_transcription() {
        let event = FrontendEvent::TranscriptionResult {
            text: "Hello world".to_string(),
            is_final: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "TranscriptionResult");
        assert_eq!(parsed["data"]["text"], "Hello world");
        assert_eq!(parsed["data"]["is_final"], true);
    }

    #[test]
    fn test_event_target_default() {
        assert_eq!(EventTarget::default(), EventTarget::Main);
    }

    #[test]
    fn test_event_dispatcher_create() {
        let dispatcher = EventDispatcher::new();
        assert!(dispatcher.is_enabled());
    }

    #[test]
    fn test_event_listener_create() {
        let (listener, tx) = EventListener::new(10);
        assert!(listener.is_running());
        drop(tx);
    }
}
