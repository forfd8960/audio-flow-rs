//! 应用状态
//!
//! 定义全局运行时状态

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 应用运行时状态
#[derive(Clone)]
pub struct AppState {
    /// 是否正在录音
    pub is_recording: Arc<AtomicBool>,
    /// 是否已连接
    pub is_connected: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            is_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn set_recording(&self, recording: bool) {
        self.is_recording.store(recording, Ordering::SeqCst);
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    pub fn set_connected(&self, connected: bool) {
        self.is_connected.store(connected, Ordering::SeqCst);
    }
}
