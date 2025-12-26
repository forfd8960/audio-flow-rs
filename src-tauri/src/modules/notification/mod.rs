//! 通知模块
//!
//! 提供系统通知功能，支持录音状态、连接状态等提示

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;
use std::sync::atomic::{AtomicBool, Ordering};

/// 通知类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    RecordingStarted,
    RecordingStopped,
    TranscriptionComplete,
    ConnectionEstablished,
    ConnectionLost,
    Error(String),
}

/// 通知管理器
#[derive(Debug)]
pub struct NotificationManager {
    enabled: AtomicBool,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }

    /// 检查通知是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// 启用/禁用通知
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// 发送录音开始通知
    pub async fn notify_recording_started<R: Runtime>(&self, app: &AppHandle<R>) {
        if !self.is_enabled() {
            return;
        }

        let _ = app.notification().builder()
            .title("AudioFlow")
            .body("开始监听...")
            .show();
    }

    /// 发送录音结束通知
    pub async fn notify_recording_stopped<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        duration: Option<std::time::Duration>,
    ) {
        if !self.is_enabled() {
            return;
        }

        let body = if let Some(d) = duration {
            format!("录音时长: {:.1}秒", d.as_secs_f64())
        } else {
            String::from("已停止录音")
        };

        let _ = app.notification().builder()
            .title("AudioFlow")
            .body(&body)
            .show();
    }

    /// 发送转写完成通知
    pub async fn notify_transcription_complete<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        text: &str,
    ) {
        if !self.is_enabled() {
            return;
        }

        // 截取前 100 个字符
        let body = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            text.to_string()
        };

        let _ = app.notification().builder()
            .title("AudioFlow - 转写完成")
            .body(&body)
            .show();
    }

    /// 发送连接建立通知
    pub async fn notify_connected<R: Runtime>(&self, app: &AppHandle<R>) {
        if !self.is_enabled() {
            return;
        }

        let _ = app.notification().builder()
            .title("AudioFlow")
            .body("已连接到语音服务")
            .show();
    }

    /// 发送连接断开通知
    pub async fn notify_disconnected<R: Runtime>(&self, app: &AppHandle<R>) {
        if !self.is_enabled() {
            return;
        }

        let _ = app.notification().builder()
            .title("AudioFlow")
            .body("已断开连接")
            .show();
    }

    /// 发送错误通知
    pub async fn notify_error<R: Runtime>(&self, app: &AppHandle<R>, error: &str) {
        if !self.is_enabled() {
            return;
        }

        let _ = app.notification().builder()
            .title("AudioFlow - 错误")
            .body(error)
            .show();
    }

    /// 通用通知
    pub async fn notify<R: Runtime>(&self, app: &AppHandle<R>, title: &str, body: &str) {
        if !self.is_enabled() {
            return;
        }

        let _ = app.notification().builder()
            .title(title)
            .body(body)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_create() {
        let manager = NotificationManager::new();
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notification_manager_toggle() {
        let manager = NotificationManager::new();
        assert!(manager.is_enabled());

        manager.set_enabled(false);
        assert!(!manager.is_enabled());

        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_notification_type_variants() {
        assert_ne!(
            NotificationType::RecordingStarted,
            NotificationType::RecordingStopped
        );
        assert_ne!(
            NotificationType::ConnectionEstablished,
            NotificationType::ConnectionLost
        );
    }
}
