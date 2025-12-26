//! AudioFlow 错误类型定义
//!
//! 所有模块的错误类型统一在此定义，使用 thiserror 自动派生 Error trait

use thiserror::Error;

/// 应用统一错误类型
#[derive(Debug, Error)]
pub enum AppError {
    /// 音频相关错误
    #[error(transparent)]
    Audio(#[from] AudioError),

    /// 网络相关错误
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// 输入相关错误
    #[error(transparent)]
    Input(#[from] InputError),

    /// 配置相关错误
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// 权限错误
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// 系统错误
    #[error("System error: {0}")]
    SystemError(String),

    /// 用户取消操作
    #[error("Operation cancelled by user")]
    Cancelled,

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 错误代码（用于前端显示）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // 音频错误 1xxx
    AudioNoDevice,
    AudioConfigFailed,
    AudioStreamFailed,
    AudioCaptureFailed,

    // 网络错误 2xxx
    NetworkConnectFailed,
    NetworkAuthFailed,
    NetworkLost,
    NetworkSendFailed,

    // 输入错误 3xxx
    InputNoWindow,
    InputPermissionDenied,
    InputInjectionFailed,
    InputClipboardFailed,

    // 配置错误 4xxx
    ConfigLoadFailed,
    ConfigSaveFailed,
    ConfigValidationFailed,
    ConfigStorageFailed,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::AudioNoDevice => write!(f, "AUDIO_NO_DEVICE"),
            ErrorCode::AudioConfigFailed => write!(f, "AUDIO_CONFIG_FAILED"),
            ErrorCode::AudioStreamFailed => write!(f, "AUDIO_STREAM_FAILED"),
            ErrorCode::AudioCaptureFailed => write!(f, "AUDIO_CAPTURE_FAILED"),
            ErrorCode::NetworkConnectFailed => write!(f, "NETWORK_CONNECT_FAILED"),
            ErrorCode::NetworkAuthFailed => write!(f, "NETWORK_AUTH_FAILED"),
            ErrorCode::NetworkLost => write!(f, "NETWORK_LOST"),
            ErrorCode::NetworkSendFailed => write!(f, "NETWORK_SEND_FAILED"),
            ErrorCode::InputNoWindow => write!(f, "INPUT_NO_WINDOW"),
            ErrorCode::InputPermissionDenied => write!(f, "INPUT_PERMISSION_DENIED"),
            ErrorCode::InputInjectionFailed => write!(f, "INPUT_INJECTION_FAILED"),
            ErrorCode::InputClipboardFailed => write!(f, "INPUT_CLIPBOARD_FAILED"),
            ErrorCode::ConfigLoadFailed => write!(f, "CONFIG_LOAD_FAILED"),
            ErrorCode::ConfigSaveFailed => write!(f, "CONFIG_SAVE_FAILED"),
            ErrorCode::ConfigValidationFailed => write!(f, "CONFIG_VALIDATION_FAILED"),
            ErrorCode::ConfigStorageFailed => write!(f, "CONFIG_STORAGE_FAILED"),
        }
    }
}

/// 音频相关错误
#[derive(Debug, PartialEq, Eq, Error)]
pub enum AudioError {
    #[error("No input device available")]
    NoDevice,

    #[error("Device configuration failed: {0}")]
    ConfigurationFailed(String),

    #[error("Stream creation failed: {0}")]
    StreamCreationFailed(String),

    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("Resampling failed: {0}")]
    ResamplingFailed(String),
}

/// 网络相关错误
#[derive(Debug, PartialEq, Eq, Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed - invalid API key")]
    AuthenticationFailed,

    #[error("Connection lost - disconnected from server")]
    ConnectionLost,

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive error: {0}")]
    ReceiveError(String),
}

/// 输入相关错误
#[derive(Debug, PartialEq, Eq, Error)]
pub enum InputError {
    #[error("No active window found")]
    NoActiveWindow,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Injection failed: {0}")]
    InjectionFailed(String),

    #[error("Clipboard operation failed")]
    ClipboardFailed,

    #[error("Failed to restore clipboard")]
    ClipboardRestoreFailed,

    #[error("Keyboard simulation failed: {0}")]
    KeyboardFailed(String),
}

/// 配置相关错误
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadFailed(String),

    #[error("Failed to save configuration: {0}")]
    SaveFailed(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),

    #[error("Secure storage failed: {0}")]
    StorageFailed(String),

    #[error("Configuration file not found")]
    NotFound,
}

/// 可恢复错误的处理策略
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 立即重试
    RetryImmediate,
    /// 指数退避重试
    RetryWithBackoff { max_retries: u32, base_delay_ms: u64 },
    /// 降级到备用方案
    Fallback(String),
    /// 提示用户操作
    UserAction(String),
    /// 无法恢复，需要重启
    Fatal,
}

impl AppError {
    /// 获取对应的错误代码
    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Audio(e) => match e {
                AudioError::NoDevice => ErrorCode::AudioNoDevice,
                AudioError::ConfigurationFailed(_) => ErrorCode::AudioConfigFailed,
                AudioError::StreamCreationFailed(_) => ErrorCode::AudioStreamFailed,
                AudioError::CaptureFailed(_) => ErrorCode::AudioCaptureFailed,
                AudioError::ResamplingFailed(_) => ErrorCode::AudioStreamFailed,
            },
            AppError::Network(e) => match e {
                NetworkError::ConnectionFailed(_) => ErrorCode::NetworkConnectFailed,
                NetworkError::AuthenticationFailed => ErrorCode::NetworkAuthFailed,
                NetworkError::ConnectionLost => ErrorCode::NetworkLost,
                NetworkError::SendFailed(_) => ErrorCode::NetworkSendFailed,
                NetworkError::ReceiveError(_) => ErrorCode::NetworkSendFailed,
            },
            AppError::Input(e) => match e {
                InputError::NoActiveWindow => ErrorCode::InputNoWindow,
                InputError::PermissionDenied(_) => ErrorCode::InputPermissionDenied,
                InputError::InjectionFailed(_) => ErrorCode::InputInjectionFailed,
                InputError::ClipboardFailed => ErrorCode::InputClipboardFailed,
                InputError::ClipboardRestoreFailed => ErrorCode::InputClipboardFailed,
                InputError::KeyboardFailed(_) => ErrorCode::InputInjectionFailed,
            },
            AppError::Config(e) => match e {
                ConfigError::LoadFailed(_) => ErrorCode::ConfigLoadFailed,
                ConfigError::SaveFailed(_) => ErrorCode::ConfigSaveFailed,
                ConfigError::ValidationFailed(_) => ErrorCode::ConfigValidationFailed,
                ConfigError::StorageFailed(_) => ErrorCode::ConfigStorageFailed,
                ConfigError::NotFound => ErrorCode::ConfigLoadFailed,
            },
            AppError::PermissionDenied(_) => ErrorCode::InputPermissionDenied,
            AppError::SystemError(_) => ErrorCode::AudioConfigFailed,
            AppError::Cancelled => ErrorCode::NetworkConnectFailed,
            AppError::Internal(_) => ErrorCode::AudioConfigFailed,
        }
    }

    /// 检查是否为可恢复错误
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            AppError::Network(NetworkError::ConnectionLost) |
            AppError::Network(NetworkError::ConnectionFailed(_))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::AudioNoDevice.to_string(), "AUDIO_NO_DEVICE");
        assert_eq!(ErrorCode::NetworkConnectFailed.to_string(), "NETWORK_CONNECT_FAILED");
        assert_eq!(ErrorCode::InputInjectionFailed.to_string(), "INPUT_INJECTION_FAILED");
        assert_eq!(ErrorCode::ConfigSaveFailed.to_string(), "CONFIG_SAVE_FAILED");
    }

    #[test]
    fn test_audio_error_display() {
        let error = AudioError::NoDevice;
        assert_eq!(error.to_string(), "No input device available");

        let error = AudioError::ConfigurationFailed("invalid format".to_string());
        assert!(error.to_string().contains("Device configuration failed"));
    }

    #[test]
    fn test_network_error_display() {
        let error = NetworkError::AuthenticationFailed;
        assert_eq!(error.to_string(), "Authentication failed - invalid API key");

        let error = NetworkError::ConnectionLost;
        assert_eq!(error.to_string(), "Connection lost - disconnected from server");
    }

    #[test]
    fn test_input_error_display() {
        let error = InputError::NoActiveWindow;
        assert_eq!(error.to_string(), "No active window found");

        let error = InputError::InjectionFailed("permission denied".to_string());
        assert!(error.to_string().contains("Injection failed"));
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::NotFound;
        assert_eq!(error.to_string(), "Configuration file not found");

        let error = ConfigError::ValidationFailed("missing field".to_string());
        assert!(error.to_string().contains("Configuration validation failed"));
    }

    #[test]
    fn test_app_error_from_audio() {
        let audio_error = AudioError::NoDevice;
        let app_error: AppError = audio_error.into();
        assert_eq!(app_error.code(), ErrorCode::AudioNoDevice);
    }

    #[test]
    fn test_app_error_from_network() {
        let network_error = NetworkError::AuthenticationFailed;
        let app_error: AppError = network_error.into();
        assert_eq!(app_error.code(), ErrorCode::NetworkAuthFailed);
    }

    #[test]
    fn test_app_error_from_input() {
        let input_error = InputError::ClipboardFailed;
        let app_error: AppError = input_error.into();
        assert_eq!(app_error.code(), ErrorCode::InputClipboardFailed);
    }

    #[test]
    fn test_app_error_from_config() {
        let config_error = ConfigError::NotFound;
        let app_error: AppError = config_error.into();
        assert_eq!(app_error.code(), ErrorCode::ConfigLoadFailed);
    }

    #[test]
    fn test_app_error_code_permission_denied() {
        let error = AppError::PermissionDenied("microphone access".to_string());
        assert_eq!(error.code(), ErrorCode::InputPermissionDenied);
    }

    #[test]
    fn test_app_error_code_system_error() {
        let error = AppError::SystemError("out of memory".to_string());
        assert_eq!(error.code(), ErrorCode::AudioConfigFailed);
    }

    #[test]
    fn test_app_error_code_cancelled() {
        let error = AppError::Cancelled;
        assert_eq!(error.code(), ErrorCode::NetworkConnectFailed);
    }

    #[test]
    fn test_app_error_is_recoverable() {
        let connection_lost = AppError::Network(NetworkError::ConnectionLost);
        assert!(connection_lost.is_recoverable());

        let connection_failed = AppError::Network(NetworkError::ConnectionFailed("timeout".to_string()));
        assert!(connection_failed.is_recoverable());

        let audio_error = AppError::Audio(AudioError::NoDevice);
        assert!(!audio_error.is_recoverable());

        let config_error = AppError::Config(ConfigError::NotFound);
        assert!(!config_error.is_recoverable());
    }

    #[test]
    fn test_recovery_strategy_variants() {
        let strategy = RecoveryStrategy::RetryImmediate;
        assert!(matches!(strategy, RecoveryStrategy::RetryImmediate));

        let strategy = RecoveryStrategy::RetryWithBackoff { max_retries: 3, base_delay_ms: 1000 };
        assert!(matches!(strategy, RecoveryStrategy::RetryWithBackoff { max_retries: 3, base_delay_ms: 1000 }));

        let strategy = RecoveryStrategy::Fallback("use default".to_string());
        assert!(matches!(strategy, RecoveryStrategy::Fallback(_)));

        let strategy = RecoveryStrategy::UserAction("grant permission".to_string());
        assert!(matches!(strategy, RecoveryStrategy::UserAction(_)));

        let strategy = RecoveryStrategy::Fatal;
        assert!(matches!(strategy, RecoveryStrategy::Fatal));
    }

    #[test]
    fn test_error_code_variants() {
        // All error codes should be unique
        let codes = vec![
            ErrorCode::AudioNoDevice,
            ErrorCode::AudioConfigFailed,
            ErrorCode::AudioStreamFailed,
            ErrorCode::AudioCaptureFailed,
            ErrorCode::NetworkConnectFailed,
            ErrorCode::NetworkAuthFailed,
            ErrorCode::NetworkLost,
            ErrorCode::NetworkSendFailed,
            ErrorCode::InputNoWindow,
            ErrorCode::InputPermissionDenied,
            ErrorCode::InputInjectionFailed,
            ErrorCode::InputClipboardFailed,
            ErrorCode::ConfigLoadFailed,
            ErrorCode::ConfigSaveFailed,
            ErrorCode::ConfigValidationFailed,
            ErrorCode::ConfigStorageFailed,
        ];

        // Check all codes are distinct
        for (i, code1) in codes.iter().enumerate() {
            for (j, code2) in codes.iter().enumerate() {
                if i != j {
                    assert_ne!(code1, code2);
                }
            }
        }
    }

    #[test]
    fn test_audio_error_variants() {
        let e1 = AudioError::NoDevice;
        let e2 = AudioError::ConfigurationFailed("test".to_string());
        let e3 = AudioError::StreamCreationFailed("test".to_string());
        let e4 = AudioError::CaptureFailed("test".to_string());
        let e5 = AudioError::ResamplingFailed("test".to_string());

        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_ne!(e3, e4);
        assert_ne!(e4, e5);
    }

    #[test]
    fn test_network_error_variants() {
        let e1 = NetworkError::ConnectionFailed("test".to_string());
        let e2 = NetworkError::AuthenticationFailed;
        let e3 = NetworkError::ConnectionLost;
        let e4 = NetworkError::SendFailed("test".to_string());
        let e5 = NetworkError::ReceiveError("test".to_string());

        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_ne!(e3, e4);
        assert_ne!(e4, e5);
    }

    #[test]
    fn test_input_error_variants() {
        let e1 = InputError::NoActiveWindow;
        let e2 = InputError::PermissionDenied("test".to_string());
        let e3 = InputError::InjectionFailed("test".to_string());
        let e4 = InputError::ClipboardFailed;
        let e5 = InputError::ClipboardRestoreFailed;
        let e6 = InputError::KeyboardFailed("test".to_string());

        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_ne!(e3, e4);
        assert_ne!(e4, e5);
        assert_ne!(e5, e6);
    }

    #[test]
    fn test_config_error_variants() {
        let e1 = ConfigError::LoadFailed("test".to_string());
        let e2 = ConfigError::SaveFailed("test".to_string());
        let e3 = ConfigError::ValidationFailed("test".to_string());
        let e4 = ConfigError::StorageFailed("test".to_string());
        let e5 = ConfigError::NotFound;

        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_ne!(e3, e4);
        assert_ne!(e4, e5);
    }
}
