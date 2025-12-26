//! WebSocket 客户端
//!
//! 使用 tokio-tungstenite 实现 WebSocket 连接管理

use crate::error::NetworkError;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::MaybeTlsStream;

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32, max_attempts: u32 },
    Failed(String),
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

impl ConnectionState {
    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting { attempt, max_attempts } => {
                write!(f, "Reconnecting ({}/{})", attempt, max_attempts)
            }
            ConnectionState::Failed(msg) => write!(f, "Failed: {}", msg),
        }
    }
}

/// WebSocket 消息
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

/// WebSocket 配置
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// 服务器 URL
    pub url: String,
    /// 连接超时 (秒)
    pub connect_timeout_secs: u64,
    /// 自动重连延迟 (毫秒)
    pub reconnect_delay_ms: u64,
    /// 最大重试次数
    pub max_reconnect_attempts: u32,
    /// 保活间隔 (秒)
    pub keep_alive_interval_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "wss://api.elevenlabs.io/v1/scribe".to_string(),
            connect_timeout_secs: 30,
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 5,
            keep_alive_interval_secs: 30,
        }
    }
}

/// WebSocket 客户端
///
/// 处理连接管理、消息收发和自动重连
#[derive(Debug)]
pub struct WebSocketClient {
    /// WebSocket 流
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    /// 连接状态
    state: Arc<Mutex<ConnectionState>>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 配置
    config: WebSocketConfig,
    /// API 密钥
    api_key: Arc<Mutex<Option<String>>>,
    /// 接收消息的通道
    message_tx: mpsc::Sender<WsMessage>,
    /// 最后活动时间
    last_activity: Arc<Mutex<Instant>>,
}

impl Default for WebSocketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketClient {
    /// 创建新的 WebSocket 客户端
    pub fn new() -> Self {
        let (message_tx, _) = mpsc::channel(100);
        Self {
            stream: None,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            running: Arc::new(AtomicBool::new(false)),
            config: WebSocketConfig::default(),
            api_key: Arc::new(Mutex::new(None)),
            message_tx,
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 创建带配置的客户端
    pub fn with_config(config: WebSocketConfig) -> Self {
        Self::new()
    }

    /// 设置 API 密钥
    pub fn set_api_key(&self, api_key: String) {
        let mut key = self.api_key.lock().unwrap();
        *key = Some(api_key);
    }

    /// 异步连接到 WebSocket 服务器
    pub async fn connect(&mut self) -> Result<(), NetworkError> {
        self.set_state(ConnectionState::Connecting);

        let url = self.config.url.clone();
        let api_key = {
            let key = self.api_key.lock().unwrap();
            key.clone().ok_or_else(|| NetworkError::AuthenticationFailed)?
        };

        // 构建带认证的 URL
        let auth_url = format!("{}?xi_api_key={}", url, api_key);

        let request = http::Request::builder()
            .uri(&auth_url)
            .header("Origin", "https://elevenlabs.io")
            .body(())
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        // 设置超时
        let timeout = Duration::from_secs(self.config.connect_timeout_secs);

        match tokio::time::timeout(timeout, connect_async(request)).await {
            Ok(Ok((stream, response))) => {
                // 验证响应状态
                if response.status() == http::StatusCode::UNAUTHORIZED {
                    return Err(NetworkError::AuthenticationFailed);
                }

                self.stream = Some(stream);
                self.set_state(ConnectionState::Connected);
                self.update_activity();
                tracing::info!("WebSocket connected to {}", self.config.url);
                Ok(())
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                self.set_state(ConnectionState::Failed(error_msg.clone()));
                Err(NetworkError::ConnectionFailed(error_msg))
            }
            Err(_) => {
                let error_msg = "Connection timed out".to_string();
                self.set_state(ConnectionState::Failed(error_msg.clone()));
                Err(NetworkError::ConnectionFailed(error_msg))
            }
        }
    }

    /// 异步断开连接
    pub async fn disconnect(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(mut stream) = self.stream.take() {
            // 发送关闭消息
            let _ = stream.close(None).await;
            tracing::info!("WebSocket disconnected");
        }

        self.set_state(ConnectionState::Disconnected);
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(*state, ConnectionState::Connected)
    }

    /// 获取当前连接状态
    pub fn connection_state(&self) -> ConnectionState {
        self.state.lock().unwrap().clone()
    }

    /// 异步发送文本消息
    pub async fn send_text(&mut self, text: &str) -> Result<(), NetworkError> {
        if let Some(ref mut stream) = self.stream {
            let message = Message::Text(text.to_string().into());
            stream.send(message).await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
            self.update_activity();
            Ok(())
        } else {
            Err(NetworkError::ConnectionLost)
        }
    }

    /// 发送二进制数据
    pub async fn send_binary(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        if let Some(ref mut stream) = self.stream {
            let message = Message::Binary(data.to_vec().into());
            stream.send(message).await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;
            self.update_activity();
            Ok(())
        } else {
            Err(NetworkError::ConnectionLost)
        }
    }

    /// 发送音频数据 (自动转换为 Base64)
    pub async fn send_audio(&mut self, audio_data: &[f32]) -> Result<(), NetworkError> {
        // 将 f32 转换为 i16，再转换为 u8 字节
        let i16_data: Vec<u8> = audio_data.iter()
            .flat_map(|&x| {
                let sample = (x.clamp(-1.0, 1.0) * 32767.0) as i16;
                sample.to_le_bytes()
            })
            .collect();

        // Base64 编码
        let b64 = STANDARD.encode(&i16_data);

        // 构造 JSON 消息
        let payload = serde_json::json!({
            "audio_base_64": b64,
            "message_type": "input_audio_chunk"
        });

        self.send_text(&payload.to_string()).await
    }

    /// 发送初始化配置
    pub async fn send_init_config(
        &mut self,
        model_id: &str,
        language_code: &str,
    ) -> Result<(), NetworkError> {
        let payload = serde_json::json!({
            "model_id": model_id,
            "language_code": language_code,
            "encoding": "pcm_16000",
            "message_type": "configure"
        });

        self.send_text(&payload.to_string()).await
    }

    /// 接收消息 (非阻塞)
    pub async fn receive(&mut self) -> Option<WsMessage> {
        if let Some(stream) = &mut self.stream {
            match stream.next().await {
                Some(Ok(message)) => {
                    self.update_activity();
                    match message {
                        Message::Text(text) => {
                            // Convert Utf8Bytes to String using as_str
                            let s = text.as_str().to_string();
                            Some(WsMessage::Text(s))
                        }
                        Message::Binary(data) => Some(WsMessage::Binary(data.to_vec())),
                        Message::Ping(data) => Some(WsMessage::Ping(data.to_vec())),
                        Message::Pong(data) => Some(WsMessage::Pong(data.to_vec())),
                        Message::Close(_) => {
                            self.stream = None;
                            Some(WsMessage::Close)
                        }
                        Message::Frame(_) => None,
                    }
                }
                Some(Err(e)) => {
                    tracing::error!("WebSocket receive error: {}", e);
                    None
                }
                None => None,
            }
        } else {
            None
        }
    }

    /// 设置连接状态
    fn set_state(&self, state: ConnectionState) {
        let mut current = self.state.lock().unwrap();
        *current = state;
    }

    /// 更新活动时间
    fn update_activity(&self) {
        let mut last = self.last_activity.lock().unwrap();
        *last = Instant::now();
    }

    /// 获取活动时间
    fn last_activity(&self) -> Instant {
        *self.last_activity.lock().unwrap()
    }
}

/// 发送消息构建器
#[derive(Debug, Default)]
pub struct MessageBuilder;

impl MessageBuilder {
    /// 构建音频消息
    pub fn audio_message(audio_data: &[f32]) -> String {
        // 将 f32 转换为 i16，再转换为 u8 字节
        let i16_data: Vec<u8> = audio_data.iter()
            .flat_map(|&x| {
                let sample = (x.clamp(-1.0, 1.0) * 32767.0) as i16;
                sample.to_le_bytes()
            })
            .collect();

        let b64 = STANDARD.encode(&i16_data);
        serde_json::json!({
            "audio_base_64": b64,
            "message_type": "input_audio_chunk"
        }).to_string()
    }

    /// 构建配置消息
    pub fn configure_message(model_id: &str, language_code: &str) -> String {
        serde_json::json!({
            "model_id": model_id,
            "language_code": language_code,
            "encoding": "pcm_16000",
            "message_type": "configure"
        }).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_display() {
        assert_eq!(ConnectionState::Connected.to_string(), "Connected");
        assert_eq!(ConnectionState::Disconnected.to_string(), "Disconnected");
        assert!(ConnectionState::Failed("error".to_string()).to_string().contains("Failed"));
    }

    #[test]
    fn test_message_builder_audio() {
        let audio = vec![0.5, -0.5, 0.25, -0.25];
        let message = MessageBuilder::audio_message(&audio);
        assert!(message.contains("audio_base_64"));
        assert!(message.contains("input_audio_chunk"));
    }

    #[test]
    fn test_message_builder_configure() {
        let message = MessageBuilder::configure_message("model_id", "en");
        assert!(message.contains("model_id"));
        assert!(message.contains("language_code"));
        assert!(message.contains("configure"));
    }
}
