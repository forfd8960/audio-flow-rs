//! 音频采集模块
//!
//! 使用 cpal 进行音频设备枚举和流采集

use crate::error::AudioError;
use cpal::{Device, Host, Stream, StreamConfig};
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 音频帧数据
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// PCM 样本值 (-1.0 到 1.0)
    pub samples: Vec<f32>,
    /// 采样率 (Hz)
    pub sample_rate: u32,
    /// 通道数
    pub channels: u16,
    /// 时间戳 (纳秒)
    pub timestamp_ns: u128,
}

impl AudioFrame {
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16, timestamp_ns: u128) -> Self {
        Self { samples, sample_rate, channels, timestamp_ns }
    }

    /// 将多通道音频混合为单通道
    pub fn to_mono(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }
        let mono: Vec<f32> = self.samples
            .chunks(self.channels as usize)
            .map(|chunk| {
                let sum: f32 = chunk.iter().sum();
                sum / self.channels as f32
            })
            .collect();
        Self::new(mono, self.sample_rate, 1, self.timestamp_ns)
    }
}

/// 音频设备信息
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    /// 设备名称
    pub name: String,
    /// 设备 ID (用于选择设备)
    pub id: String,
    /// 支持的采样率
    pub sample_rates: Vec<u32>,
    /// 最大通道数
    pub channels: u16,
}

/// 音频配置
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// 设备 ID，None 表示使用默认设备
    pub device_id: Option<String>,
    /// 目标采样率
    pub sample_rate: u32,
    /// 通道数
    pub channels: u16,
    /// 缓冲区大小 (毫秒)
    pub buffer_size_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            sample_rate: 48000,
            channels: 1,
            buffer_size_ms: 20,
        }
    }
}

/// 环形缓冲区用于线程间音频数据传输
#[derive(Debug)]
pub struct RingBuffer {
    buffer: Arc<Mutex<Vec<f32>>>,
    capacity: usize,
    write_pos: Arc<AtomicUsize>,
    read_pos: Arc<AtomicUsize>,
}

impl RingBuffer {
    /// 创建新的环形缓冲区
    pub fn new(capacity_samples: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(vec![0.0; capacity_samples])),
            capacity: capacity_samples,
            write_pos: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 写入数据
    pub fn write(&self, data: &[f32]) -> usize {
        let mut buffer = self.buffer.lock().unwrap();
        let write_pos = self.write_pos.load(Ordering::SeqCst);
        let read_pos = self.read_pos.load(Ordering::SeqCst);

        let available = self.capacity.saturating_sub(
            (write_pos as i64 - read_pos as i64 + self.capacity as i64) as usize % self.capacity
        );
        let to_write = data.len().min(available.saturating_sub(1));

        for i in 0..to_write {
            let idx = (write_pos + i) % self.capacity;
            buffer[idx] = data[i];
        }

        if to_write > 0 {
            self.write_pos.store((write_pos + to_write) % self.capacity, Ordering::SeqCst);
        }
        to_write
    }

    /// 读取数据
    pub fn read(&self, size: usize) -> Option<Vec<f32>> {
        let mut buffer = self.buffer.lock().unwrap();
        let write_pos = self.write_pos.load(Ordering::SeqCst);
        let read_pos = self.read_pos.load(Ordering::SeqCst);

        let available = (write_pos as i64 - read_pos as i64 + self.capacity as i64) as usize % self.capacity;
        if available == 0 {
            return None;
        }

        let to_read = size.min(available);
        let mut result = Vec::with_capacity(to_read);

        for i in 0..to_read {
            let idx = (read_pos + i) % self.capacity;
            result.push(buffer[idx]);
        }

        self.read_pos.store((read_pos + to_read) % self.capacity, Ordering::SeqCst);
        Some(result)
    }

    /// 获取可用数据量
    pub fn available(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::SeqCst);
        let read_pos = self.read_pos.load(Ordering::SeqCst);
        (write_pos as i64 - read_pos as i64 + self.capacity as i64) as usize % self.capacity
    }

    /// 清空缓冲区
    pub fn clear(&self) {
        let mut buffer = self.buffer.lock().unwrap();
        self.write_pos.store(0, Ordering::SeqCst);
        self.read_pos.store(0, Ordering::SeqCst);
        buffer.fill(0.0);
    }
}

/// 音频采集器
pub struct AudioCapturer {
    host: Host,
    device: Option<Device>,
    stream: Option<Stream>,
    config: AudioConfig,
    is_running: Arc<AtomicBool>,
    ring_buffer: Arc<RingBuffer>,
}

impl Default for AudioCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCapturer {
    /// 创建新的音频采集器
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self {
            host,
            device: None,
            stream: None,
            config: AudioConfig::default(),
            is_running: Arc::new(AtomicBool::new(false)),
            ring_buffer: Arc::new(RingBuffer::new(48000 * 2)), // 2秒缓冲
        }
    }

    /// 获取所有可用输入设备
    pub fn available_devices() -> Result<Vec<AudioDeviceInfo>, AudioError> {
        let host = cpal::default_host();
        let devices = host.input_devices()
            .map_err(|e| AudioError::ConfigurationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for device in devices {
            if let Ok(info) = build_device_info(&device) {
                result.push(info);
            }
        }
        Ok(result)
    }

    /// 获取默认输入设备
    pub fn default_device() -> Result<AudioDeviceInfo, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or(AudioError::NoDevice)?;
        build_device_info(&device)
    }

    /// 配置采集器
    pub fn configure(&mut self, config: AudioConfig) -> Result<(), AudioError> {
        self.config = config.clone();

        // 选择设备
        self.device = if let Some(id) = &self.config.device_id {
            let mut devices = self.host.input_devices()
                .map_err(|e| AudioError::ConfigurationFailed(e.to_string()))?;
            devices.find(|d| d.name().map(|n| n == *id).unwrap_or(false))
        } else {
            self.host.default_input_device()
        };

        let device = self.device.as_ref()
            .ok_or(AudioError::NoDevice)?;

        // 验证设备支持配置
        let _supported_config = device.default_input_config()
            .map_err(|e| AudioError::ConfigurationFailed(e.to_string()))?;

        // 简化：直接使用默认配置
        // 在实际应用中可能需要更复杂的采样率协商

        Ok(())
    }

    /// 启动音频采集
    pub fn start(&mut self) -> Result<(), AudioError> {
        if self.stream.is_some() {
            return Ok(());
        }

        let device = self.device.as_ref()
            .ok_or(AudioError::NoDevice)?;

        let config = StreamConfig {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(
                (self.config.sample_rate * self.config.buffer_size_ms / 1000) as u32
            ),
        };

        let is_running = self.is_running.clone();
        let ring_buffer = self.ring_buffer.clone();

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                if is_running.load(Ordering::SeqCst) {
                    // 将数据写入环形缓冲区
                    ring_buffer.write(data);
                }
            },
            move |err| {
                tracing::error!("Audio stream error: {}", err);
            },
            None,
        ).map_err(|e| AudioError::StreamCreationFailed(e.to_string()))?;

        stream.play()
            .map_err(|e| AudioError::StreamCreationFailed(e.to_string()))?;

        self.is_running.store(true, Ordering::SeqCst);
        self.stream = Some(stream);

        tracing::info!("Audio capture started: {}Hz, {} channels",
            self.config.sample_rate, self.config.channels);

        Ok(())
    }

    /// 停止音频采集
    pub fn stop(&mut self) -> Result<(), AudioError> {
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(stream) = self.stream.take() {
            drop(stream);
            tracing::info!("Audio capture stopped");
        }

        Ok(())
    }

    /// 检查是否正在采集
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// 读取音频帧
    pub fn read_frame(&self, max_samples: usize) -> Option<AudioFrame> {
        let samples = self.ring_buffer.read(max_samples)?;
        let now = std::time::Instant::now();
        Some(AudioFrame::new(
            samples,
            self.config.sample_rate,
            self.config.channels,
            now.elapsed().as_nanos(),
        ))
    }

    /// 获取环形缓冲区引用（用于外部访问）
    pub fn ring_buffer(&self) -> &RingBuffer {
        &self.ring_buffer
    }
}

/// 构建设备信息
fn build_device_info(device: &Device) -> Result<AudioDeviceInfo, AudioError> {
    let name = device.name()
        .map_err(|e| AudioError::ConfigurationFailed(e.to_string()))?;

    let default_config = device.default_input_config()
        .map_err(|e| AudioError::ConfigurationFailed(e.to_string()))?;

    let channels = default_config.channels();
    let sample_rate: u32 = default_config.sample_rate();

    Ok(AudioDeviceInfo {
        name: name.clone(),
        id: name,
        sample_rates: vec![sample_rate],
        channels,
    })
}

impl Drop for AudioCapturer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
