//! 重采样模块
//!
//! 使用 rubato 库实现高质量音频重采样

use crate::error::AudioError;
use rubato::{Resampler, FastFixedIn, PolynomialDegree};
use std::error::Error;

/// 音频重采样器
///
/// 使用多项式插值实现轻量级重采样
pub struct AudioResampler {
    /// 内部重采样器
    resampler: Option<FastFixedIn<f32>>,
    /// 输入采样率
    input_rate: u32,
    /// 输出采样率
    output_rate: u32,
    /// 单次处理的输入帧数
    chunk_size: usize,
}

impl AudioResampler {
    /// 创建新的重采样器
    ///
    /// # Arguments
    /// * `input_rate` - 输入采样率
    /// * `output_rate` - 输出采样率
    ///
    /// # Returns
    /// 新的重采样器实例
    pub fn new(input_rate: u32, output_rate: u32) -> Result<Self, AudioError> {
        if input_rate == output_rate {
            return Ok(Self {
                resampler: None,
                input_rate,
                output_rate,
                chunk_size: 0,
            });
        }

        // 使用三次多项式插值
        let resampler = FastFixedIn::new(
            input_rate as f64,
            output_rate as f64,
            PolynomialDegree::Cubic,
            128,  // chunk_size
            128,  // output_chunk_size
        ).map_err(|e| AudioError::ResamplingFailed(e.to_string()))?;

        Ok(Self {
            resampler: Some(resampler),
            input_rate,
            output_rate,
            chunk_size: 128,
        })
    }

    /// 创建 48kHz → 16kHz 重采样器
    pub fn create_48k_to_16k() -> Result<Self, AudioError> {
        Self::new(48000, 16000)
    }

    /// 重采样音频数据
    ///
    /// # Arguments
    /// * `input` - 输入音频数据
    ///
    /// # Returns
    /// 重采样后的音频数据
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, AudioError> {
        if self.resampler.is_none() {
            // 无需重采样，直接返回
            return Ok(input.to_vec());
        }

        let resampler = self.resampler.as_mut().unwrap();
        let mut output = Vec::new();

        // rubato expects &[V] where V: AsRef<[T]>, so we wrap input in a vec
        let input_vec: Vec<&[f32]> = vec![input];

        // process returns Result<Vec<Vec<T>, ResampleError>
        let result: Result<Vec<Vec<f32>>, _> = resampler.process(&input_vec, None);
        let chunks = result.map_err(|e| AudioError::ResamplingFailed(e.to_string()))?;

        // Flatten the result (for mono audio, chunks has one channel)
        for chunk in chunks {
            output.extend(chunk);
        }

        Ok(output)
    }

    /// 获取输入采样率
    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }

    /// 获取输出采样率
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }

    /// 检查是否需要重采样
    pub fn needs_resampling(&self) -> bool {
        self.input_rate != self.output_rate
    }
}

/// 批量重采样器
///
/// 用于处理大量音频数据的分块重采样
pub struct BatchResampler {
    resampler: AudioResampler,
    buffer: Vec<f32>,
}

impl BatchResampler {
    /// 创建新的批量重采样器
    pub fn new(input_rate: u32, output_rate: u32) -> Result<Self, AudioError> {
        let resampler = AudioResampler::new(input_rate, output_rate)?;
        Ok(Self {
            resampler,
            buffer: Vec::new(),
        })
    }

    /// 添加数据并处理
    ///
    /// 当缓冲区积累到足够数据时进行处理
    pub fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, AudioError> {
        self.buffer.extend_from_slice(input);

        let chunk_size = self.resampler.chunk_size;
        let mut output = Vec::new();

        while self.buffer.len() >= chunk_size {
            let chunk = self.buffer[..chunk_size].to_vec();
            let processed = self.resampler.process(&chunk)?;
            output.extend_from_slice(&processed);

            self.buffer.drain(..chunk_size);
        }

        Ok(output)
    }

    /// 刷新缓冲区，获取剩余数据
    pub fn flush(&mut self) -> Result<Vec<f32>, AudioError> {
        let mut output = Vec::new();

        if !self.buffer.is_empty() {
            // 对最后一块进行零填充
            let chunk_size = self.resampler.chunk_size;
            let mut chunk = self.buffer.clone();
            chunk.resize(chunk_size, 0.0);

            let processed = self.resampler.process(&chunk)?;
            output.extend_from_slice(&processed);

            self.buffer.clear();
        }

        Ok(output)
    }
}

impl Default for AudioResampler {
    fn default() -> Self {
        Self::new(48000, 16000).unwrap_or_else(|_| Self {
            resampler: None,
            input_rate: 48000,
            output_rate: 16000,
            chunk_size: 128,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_resample_needed() {
        let mut resampler = AudioResampler::new(16000, 16000).unwrap();
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let output = resampler.process(&input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_resample_rates() {
        let resampler = AudioResampler::new(48000, 16000).unwrap();
        assert_eq!(resampler.input_rate(), 48000);
        assert_eq!(resampler.output_rate(), 16000);
        assert!(resampler.needs_resampling());
    }

    #[test]
    fn test_same_rates_no_resampling() {
        let resampler = AudioResampler::new(48000, 48000).unwrap();
        assert!(!resampler.needs_resampling());
    }
}
