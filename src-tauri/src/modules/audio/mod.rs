//! 音频处理模块
//!
//! 提供音频采集、重采样和语音活动检测功能

pub mod capture;
pub mod resampler;
pub mod vad;

pub use capture::{AudioCapturer, AudioConfig, AudioDeviceInfo, AudioFrame, RingBuffer};
pub use resampler::{AudioResampler, BatchResampler};
pub use vad::{VadConfig, VadState, VoiceActivityDetector};
