//! 语音活动检测模块
//!
//! 使用能量检测进行语音活动识别

use std::f32::consts::LN_10;

/// VAD 配置
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// 语音检测阈值 (dB)，低于此值认为是静音
    /// 默认 -50 dB 适合安静环境，-40 dB 适合嘈杂环境
    pub threshold_db: f32,
    /// 平滑因子 (0.0 - 1.0)，用于平滑能量计算
    /// 值越大平滑越强，但响应延迟越大
    pub smoothing_factor: f32,
    /// 静音超时时间 (帧数)，检测到静音后多少帧认为语音结束
    pub silence_timeout_frames: usize,
    /// 最小语音帧数，低于此帧数的语音片段被忽略
    pub min_speech_frames: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            threshold_db: -50.0,
            smoothing_factor: 0.3,
            silence_timeout_frames: 15,  // 约 300ms @ 20ms/帧
            min_speech_frames: 3,        // 约 60ms
        }
    }
}

/// VAD 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// 静音状态
    Silence,
    /// 检测到语音
    Speech,
    /// 语音结束检测中
    Ending,
}

/// 语音活动检测器
///
/// 使用短时能量 (Short-Time Energy) 进行语音检测
#[derive(Debug)]
pub struct VoiceActivityDetector {
    config: VadConfig,
    /// 平滑后的能量值 (线性)
    smoothed_energy: f32,
    /// 连续静音帧数
    silence_frames: usize,
    /// 连续语音帧数
    speech_frames: usize,
    /// 当前状态
    state: VadState,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(VadConfig::default())
    }
}

impl VoiceActivityDetector {
    /// 创建新的 VAD 实例
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            smoothed_energy: 0.0,
            silence_frames: 0,
            speech_frames: 0,
            state: VadState::Silence,
        }
    }

    /// 检测音频帧中是否包含语音
    ///
    /// # Arguments
    /// * `frame` - 音频样本数据 (f32, -1.0 到 1.0)
    ///
    /// # Returns
    /// VAD 状态
    pub fn detect(&mut self, frame: &[f32]) -> VadState {
        // 计算当前帧的 RMS 能量
        let energy = self.calculate_energy(frame);

        // 应用平滑 - 先保存旧值用于平滑计算
        let old_smoothed = self.smoothed_energy;
        self.smoothed_energy = self.config.smoothing_factor * energy
            + (1.0 - self.config.smoothing_factor) * old_smoothed;

        // 对于语音检测，使用当前帧的能量加上平滑后的能量
        // 这样即使 smoothing_factor 为 0，也能检测到语音
        let detection_energy = if self.config.smoothing_factor > 0.0 {
            self.smoothed_energy
        } else {
            energy
        };

        // 转换为 dB
        let dbfs = self.energy_to_dbfs(detection_energy);

        // 判断是否超过阈值
        let is_speech = dbfs > self.config.threshold_db;

        // 状态机更新
        match self.state {
            VadState::Silence => {
                if is_speech {
                    self.speech_frames = 1;
                    self.silence_frames = 0;
                    self.state = VadState::Speech;
                }
            }
            VadState::Speech => {
                if is_speech {
                    self.speech_frames += 1;
                    self.silence_frames = 0;
                } else {
                    self.silence_frames += 1;
                    if self.silence_frames >= self.config.silence_timeout_frames {
                        if self.speech_frames >= self.config.min_speech_frames {
                            self.state = VadState::Ending;
                        } else {
                            // 语音太短，忽略
                            self.state = VadState::Silence;
                        }
                        self.speech_frames = 0;
                    }
                }
            }
            VadState::Ending => {
                // 返回 Ending 状态一次后回到 Silence
                self.state = VadState::Silence;
                self.silence_frames = 0;
            }
        }

        self.state
    }

    /// 计算音频帧的 RMS 能量
    fn calculate_energy(&self, frame: &[f32]) -> f32 {
        if frame.is_empty() {
            return 0.0;
        }

        // 计算 RMS
        let sum_squares: f32 = frame.iter()
            .map(|&x| x * x)
            .sum();

        sum_squares / frame.len() as f32
    }

    /// 将线性能量转换为 dBFS
    fn energy_to_dbfs(&self, energy: f32) -> f32 {
        if energy <= 0.0 {
            return f32::NEG_INFINITY;
        }
        20.0 * energy.log10()
    }

    /// 重置 VAD 状态
    pub fn reset(&mut self) {
        self.smoothed_energy = 0.0;
        self.silence_frames = 0;
        self.speech_frames = 0;
        self.state = VadState::Silence;
    }

    /// 获取当前状态
    pub fn state(&self) -> VadState {
        self.state
    }

    /// 获取当前能量 (dB)
    pub fn energy_db(&self) -> f32 {
        self.energy_to_dbfs(self.smoothed_energy)
    }

    /// 检查是否正在语音中
    pub fn is_speaking(&self) -> bool {
        matches!(self.state, VadState::Speech)
    }

    /// 获取连续语音帧数
    pub fn speech_frame_count(&self) -> usize {
        self.speech_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence_detection() {
        let config = VadConfig {
            threshold_db: -50.0,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config);

        // 静音帧 (接近 0 的值)
        let silence_frame = vec![0.0001; 480];
        let state = vad.detect(&silence_frame);
        assert_eq!(state, VadState::Silence);
    }

    #[test]
    fn test_vad_speech_detection() {
        let config = VadConfig {
            threshold_db: -50.0,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config);

        // 语音帧 (较高的振幅)
        let speech_frame = vec![0.5; 480];
        let state = vad.detect(&speech_frame);
        assert_eq!(state, VadState::Speech);
    }

    #[test]
    fn test_vad_state_transitions() {
        let config = VadConfig {
            threshold_db: -50.0,
            silence_timeout_frames: 2,
            min_speech_frames: 1,
            smoothing_factor: 0.0,
        };
        let mut vad = VoiceActivityDetector::new(config);

        // 初始状态
        assert_eq!(vad.state(), VadState::Silence);

        // 开始说话
        let speech_frame = vec![0.5; 480];
        assert_eq!(vad.detect(&speech_frame), VadState::Speech);

        // 停止说话 (1帧静音)
        let silence_frame = vec![0.0001; 480];
        assert_eq!(vad.detect(&silence_frame), VadState::Speech);

        // 再停止一帧 (超时)
        assert_eq!(vad.detect(&silence_frame), VadState::Ending);

        // 下一帧回到 Silence
        assert_eq!(vad.detect(&silence_frame), VadState::Silence);
    }

    #[test]
    fn test_vad_reset() {
        let config = VadConfig::default();
        let mut vad = VoiceActivityDetector::new(config);

        // 说点话
        let speech_frame = vec![0.5; 480];
        vad.detect(&speech_frame);
        assert!(vad.is_speaking());

        // 重置
        vad.reset();
        assert_eq!(vad.state(), VadState::Silence);
        assert!(!vad.is_speaking());
    }

    #[test]
    fn test_energy_calculation() {
        let config = VadConfig::default();
        let vad = VoiceActivityDetector::new(config);

        // 静音帧
        let silence = vec![0.0; 480];
        let energy = vad.calculate_energy(&silence);
        assert_eq!(energy, 0.0);

        // 0.5 幅度的正弦波
        let speech = vec![0.5; 480];
        let energy = vad.calculate_energy(&speech);
        // RMS of constant 0.5 = 0.5
        assert!((energy - 0.25).abs() < 0.0001);
    }
}
