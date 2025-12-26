//! 事件模块
//!
//! 定义应用事件和状态，以及事件分发器

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// 应用状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AppState {
    Idle,
    Connecting,
    Listening,
    Transcribing,
    Injecting,
    Error(String),
}

impl fmt::Display for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppState::Idle => write!(f, "Idle"),
            AppState::Connecting => write!(f, "Connecting"),
            AppState::Listening => write!(f, "Listening"),
            AppState::Transcribing => write!(f, "Transcribing"),
            AppState::Injecting => write!(f, "Injecting"),
            AppState::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// 前端事件载荷
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type")]
pub enum FrontendEvent {
    StateChanged { old_state: AppState, new_state: AppState },
    AudioLevel { level: f32, peak: f32 },
    PartialTranscript { text: String, timestamp: DateTime<Utc> },
    CommittedTranscript { text: String, confidence: f64, timestamp: DateTime<Utc> },
    Error { code: String, message: String, recoverable: bool },
    ConfigUpdated,
    RecordingState { is_recording: bool },
    SessionStarted { session_id: String },
    ConnectionStateChanged { state: String },
}

/// 事件发射器
#[derive(Debug)]
pub struct EventEmitter {
    app: AppHandle,
}

impl EventEmitter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn emit(&self, event: &str, payload: impl Serialize + Clone) {
        let _ = self.app.emit(event, payload);
    }

    pub fn emit_state_change(&self, old_state: &AppState, new_state: &AppState) {
        self.emit("state_changed", StateChangePayload {
            old_state: old_state.clone(),
            new_state: new_state.clone(),
        });
    }

    pub fn emit_audio_level(&self, level: f32, peak: f32) {
        self.emit("audio_level", AudioLevelPayload { level, peak });
    }

    pub fn emit_partial_transcript(&self, text: &str) {
        self.emit("partial_transcript", PartialTranscriptPayload {
            text: text.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn emit_committed_transcript(&self, text: &str, confidence: f64) {
        self.emit("committed_transcript", CommittedTranscriptPayload {
            text: text.to_string(),
            confidence,
            timestamp: Utc::now(),
        });
    }

    pub fn emit_error(&self, code: &str, message: &str, recoverable: bool) {
        self.emit("error", ErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            recoverable,
        });
    }

    pub fn emit_recording_state(&self, is_recording: bool) {
        self.emit("recording_state", RecordingStatePayload { is_recording });
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StateChangePayload {
    pub old_state: AppState,
    pub new_state: AppState,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelPayload {
    pub level: f32,
    pub peak: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartialTranscriptPayload {
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommittedTranscriptPayload {
    pub text: String,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingStatePayload {
    pub is_recording: bool,
}

/// 事件分发器
///
/// 将内部事件转发到前端
#[derive(Debug)]
pub struct EventDispatcher {
    /// Tauri 应用句柄
    app: AppHandle,
    /// 是否正在运行
    running: Arc<AtomicBool>,
}

impl EventDispatcher {
    /// 创建新的事件分发器
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 发送事件到前端
    pub fn emit(&self, event: &FrontendEvent) {
        Self::emit_event(&self.app, event);
    }

    /// 发送状态改变事件
    pub fn emit_state_change(&self, old_state: &AppState, new_state: &AppState) {
        self.emit(&FrontendEvent::StateChanged {
            old_state: old_state.clone(),
            new_state: new_state.clone(),
        });
    }

    /// 发送音频级别事件
    pub fn emit_audio_level(&self, level: f32, peak: f32) {
        self.emit(&FrontendEvent::AudioLevel { level, peak });
    }

    /// 发送部分转写事件
    pub fn emit_partial_transcript(&self, text: &str) {
        self.emit(&FrontendEvent::PartialTranscript {
            text: text.to_string(),
            timestamp: Utc::now(),
        });
    }

    /// 发送完整转写事件
    pub fn emit_committed_transcript(&self, text: &str, confidence: f64) {
        self.emit(&FrontendEvent::CommittedTranscript {
            text: text.to_string(),
            confidence,
            timestamp: Utc::now(),
        });
    }

    /// 发送错误事件
    pub fn emit_error(&self, code: &str, message: &str, recoverable: bool) {
        self.emit(&FrontendEvent::Error {
            code: code.to_string(),
            message: message.to_string(),
            recoverable,
        });
    }

    /// 发送录音状态事件
    pub fn emit_recording_state(&self, is_recording: bool) {
        self.emit(&FrontendEvent::RecordingState { is_recording });
    }

    /// 启动事件分发 (保留用于异步任务)
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// 停止事件分发
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// 发送事件到前端
    fn emit_event(app: &AppHandle, event: &FrontendEvent) {
        let event_name = match event {
            FrontendEvent::StateChanged { .. } => "state_changed",
            FrontendEvent::AudioLevel { .. } => "audio_level",
            FrontendEvent::PartialTranscript { .. } => "partial_transcript",
            FrontendEvent::CommittedTranscript { .. } => "committed_transcript",
            FrontendEvent::Error { .. } => "error",
            FrontendEvent::ConfigUpdated => "config_updated",
            FrontendEvent::RecordingState { .. } => "recording_state",
            FrontendEvent::SessionStarted { .. } => "session_started",
            FrontendEvent::ConnectionStateChanged { .. } => "connection_state_changed",
        };

        if let Err(e) = app.emit(event_name, event.clone()) {
            tracing::error!("Failed to emit event {}: {}", event_name, e);
        }
    }
}
