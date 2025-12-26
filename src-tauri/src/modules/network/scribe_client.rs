//! Scribe API 客户端
//!
//! 封装 WebSocket 客户端，提供 ElevenLabs Scribe 语音转写功能

use crate::error::NetworkError;
use crate::modules::audio::VadLevel;
use crate::modules::network::websocket::{ConnectionState, WebSocketClient, WebSocketConfig, WsMessage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Scribe 配置
#[derive(Debug, Clone)]
pub struct ScribeConfig {
    /// API 端点
    pub endpoint: String,
    /// 模型 ID
    pub model_id: String,
    /// 语言代码
    pub language_code: String,
    /// API 密钥
    pub api_key: Option<String>,
}

impl Default for ScribeConfig {
    fn default() -> Self {
        Self {
            endpoint: "wss://api.elevenlabs.io/v1/scribe".to_string(),
            model_id: "scribe_v1".to_string(),
            language_code: "en".to_string(),
            api_key: None,
        }
    }
}

/// Scribe 事件
///
/// 从 WebSocket 接收的转写事件
#[derive(Debug, Clone, Serialize)]
pub enum ScribeEvent {
    /// 会话已启动
    #[serde(rename = "session_started")]
    SessionStarted {
        session_id: String,
        timestamp: DateTime<Utc>,
    },

    /// 部分转写结果
    #[serde(rename = "partial_transcript")]
    PartialTranscript {
        text: String,
        timestamp: DateTime<Utc>,
    },

    /// 完整转写结果
    #[serde(rename = "committed_transcript")]
    CommittedTranscript {
        text: String,
        confidence: f64,
        timestamp: DateTime<Utc>,
    },

    /// Word-level 结果
    #[serde(rename = "word_details")]
    WordDetails {
        text: String,
        start_ms: i64,
        end_ms: i64,
        confidence: f64,
    },

    /// 错误事件
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
    },

    /// 连接断开
    Disconnected,
}

/// 转写结果
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    /// 最终文本
    pub text: String,
    /// 置信度
    pub confidence: f64,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 是否为最终结果
    pub is_final: bool,
}

/// Scribe 客户端
///
/// 高级 API 客户端，处理会话管理和事件分发
#[derive(Debug)]
pub struct ScribeClient {
    /// WebSocket 客户端
    ws_client: WebSocketClient,
    /// 配置
    config: ScribeConfig,
    /// 事件发送通道
    event_tx: mpsc::Sender<ScribeEvent>,
    /// 事件接收通道
    event_rx: mpsc::Receiver<ScribeEvent>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 会话 ID
    session_id: Arc<Mutex<Option<String>>>,
    /// 最后转写文本 (用于去重)
    last_transcript: Arc<Mutex<Option<String>>>,
    /// 累计的 partial transcript
    partial_buffer: Arc<Mutex<String>>,
}

impl Default for ScribeClient {
    fn default() -> Self {
        Self::new(ScribeConfig::default())
    }
}

impl ScribeClient {
    /// 创建新的 Scribe 客户端
    pub fn new(config: ScribeConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);

        // 配置 WebSocket
        let ws_config = WebSocketConfig {
            url: config.endpoint.clone(),
            ..Default::default()
        };

        Self {
            ws_client: WebSocketClient::with_config(ws_config),
            config: config.clone(),
            event_tx,
            event_rx,
            running: Arc::new(AtomicBool::new(false)),
            session_id: Arc::new(Mutex::new(None)),
            last_transcript: Arc::new(Mutex::new(None)),
            partial_buffer: Arc::new(Mutex::new(String::new())),
        }
    }

    /// 设置 API 密钥
    pub fn set_api_key(&mut self, api_key: String) {
        self.config.api_key = Some(api_key.clone());
        self.ws_client.set_api_key(api_key);
    }

    /// 连接到 Scribe 服务
    pub async fn connect(&mut self) -> Result<(), NetworkError> {
        if let Some(api_key) = &self.config.api_key {
            self.ws_client.set_api_key(api_key.clone());
        }

        self.ws_client.connect().await?;

        // 发送配置
        self.ws_client.send_init_config(
            &self.config.model_id,
            &self.config.language_code,
        ).await?;

        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.ws_client.disconnect().await;

        // 清空 partial buffer
        let mut buffer = self.partial_buffer.lock().unwrap();
        buffer.clear();
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.ws_client.is_connected()
    }

    /// 获取连接状态
    pub fn connection_state(&self) -> ConnectionState {
        self.ws_client.connection_state()
    }

    /// 发送音频数据
    pub async fn send_audio(&mut self, audio_data: &[f32]) -> Result<(), NetworkError> {
        self.ws_client.send_audio(audio_data).await
    }

    /// 开始转写会话
    pub async fn start_session(&mut self) -> Result<(), NetworkError> {
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 停止转写会话
    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        self.running.store(false, Ordering::SeqCst);
        self.ws_client.disconnect().await;
        Ok(())
    }

    /// 接收事件 (非阻塞)
    pub async fn receive_event(&mut self) -> Option<ScribeEvent> {
        if let Some(msg) = self.ws_client.receive().await {
            Some(self.parse_message(msg).await)
        } else {
            None
        }
    }

    /// 异步任务：处理消息循环
    pub async fn run(&mut self) {
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            if let Some(event) = self.receive_event().await {
                // 发送事件到通道
                let _ = self.event_tx.send(event).await;
            }
            // 短暂休眠，避免 busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// 尝试接收事件 (带超时)
    pub async fn try_receive(&mut self) -> Option<ScribeEvent> {
        let result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            self.event_rx.recv()
        ).await;

        match result {
            Ok(Some(event)) => Some(event),
            _ => None,
        }
    }

    /// 解析 WebSocket 消息
    async fn parse_message(&self, msg: WsMessage) -> ScribeEvent {
        match msg {
            WsMessage::Text(text) => {
                self.parse_text_message(&text).await
            }
            WsMessage::Close => ScribeEvent::Disconnected,
            _ => ScribeEvent::Disconnected,
        }
    }

    /// 解析文本消息
    async fn parse_text_message(&self, text: &str) -> ScribeEvent {
        // 尝试解析为已知的事件类型
        if let Ok(response) = serde_json::from_str::<serde_json::Value>(text) {
            let message_type = response.get("message_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let timestamp = Utc::now();

            match message_type {
                "session_started" => {
                    let session_id = response.get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let mut sid = self.session_id.lock().unwrap();
                    *sid = Some(session_id.clone());
                    ScribeEvent::SessionStarted { session_id, timestamp }
                }

                "partial_transcript" => {
                    let text = response.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // 更新 partial buffer
                    let mut buffer = self.partial_buffer.lock().unwrap();
                    *buffer = text.clone();

                    ScribeEvent::PartialTranscript { text, timestamp }
                }

                "committed_transcript" => {
                    let text = response.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let confidence = response.get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    // 清空 partial buffer
                    let mut buffer = self.partial_buffer.lock().unwrap();
                    buffer.clear();

                    // 更新 last transcript
                    let mut last = self.last_transcript.lock().unwrap();
                    *last = Some(text.clone());

                    ScribeEvent::CommittedTranscript {
                        text,
                        confidence,
                        timestamp,
                    }
                }

                "error" => {
                    let code = response.get("code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let message = response.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error")
                        .to_string();

                    ScribeEvent::Error { code, message }
                }

                _ => {
                    tracing::debug!("Unknown message type: {}", message_type);
                    ScribeEvent::Error {
                        code: "unknown_type".to_string(),
                        message: format!("Unknown message type: {}", message_type),
                    }
                }
            }
        } else {
            ScribeEvent::Error {
                code: "parse_error".to_string(),
                message: "Failed to parse message".to_string(),
            }
        }
    }

    /// 获取当前 partial transcript
    pub fn current_partial(&self) -> String {
        self.partial_buffer.lock().unwrap().clone()
    }

    /// 获取 session ID
    pub fn session_id(&self) -> Option<String> {
        self.session_id.lock().unwrap().clone()
    }

    /// 接收转写响应 (供命令使用)
    pub async fn receive_response(&mut self) -> Result<Option<TranscriptionResult>, NetworkError> {
        if let Some(event) = self.receive_event().await {
            match event {
                ScribeEvent::PartialTranscript { text, .. } => {
                    Ok(Some(TranscriptionResult {
                        text,
                        confidence: 0.0,
                        timestamp: Utc::now(),
                        is_final: false,
                    }))
                }
                ScribeEvent::CommittedTranscript { text, confidence, .. } => {
                    Ok(Some(TranscriptionResult {
                        text,
                        confidence,
                        timestamp: Utc::now(),
                        is_final: true,
                    }))
                }
                ScribeEvent::Error { message, .. } => {
                    Err(NetworkError::ReceiveError(message))
                }
                ScribeEvent::Disconnected => {
                    Err(NetworkError::ConnectionLost)
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: ScribeConfig) {
        self.config = config;
    }

    /// VAD 级别
    pub fn vad_level(&self) -> VadLevel {
        // 默认返回 Balanced，实际实现可以根据配置调整
        VadLevel::Balanced
    }

    /// 设置 VAD 级别
    pub fn set_vad_level(&mut self, _level: VadLevel) {
        // VAD 设置在 WebSocket 消息中发送
        // 实际实现需要在配置时发送 VAD 设置
    }
}

/// 转写响应解析器
#[derive(Debug, Default)]
pub struct TranscriptionParser;

impl TranscriptionParser {
    /// 解析 partial transcript 响应
    pub fn parse_partial(text: &str) -> Option<String> {
        serde_json::from_str::<PartialTranscriptResponse>(text)
            .ok()
            .map(|r| r.text)
    }

    /// 解析 committed transcript 响应
    pub fn parse_committed(text: &str) -> Option<TranscriptionResult> {
        serde_json::from_str::<CommittedTranscriptResponse>(text)
            .ok()
            .map(|r| TranscriptionResult {
                text: r.text,
                confidence: r.confidence.unwrap_or(1.0),
                timestamp: Utc::now(),
                is_final: true,
            })
    }
}

/// Partial transcript 响应结构
#[derive(Debug, Deserialize)]
struct PartialTranscriptResponse {
    text: String,
    message_type: String,
}

/// Committed transcript 响应结构
#[derive(Debug, Deserialize)]
struct CommittedTranscriptResponse {
    text: String,
    confidence: Option<f64>,
    message_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scribe_config_default() {
        let config = ScribeConfig::default();
        assert_eq!(config.model_id, "scribe_v1");
        assert_eq!(config.language_code, "en");
    }

    #[test]
    fn test_transcription_parser_partial() {
        let json = r#"{"text": "hello world", "message_type": "partial_transcript"}"#;
        let result = TranscriptionParser::parse_partial(json);
        assert_eq!(result, Some("hello world".to_string()));
    }

    #[test]
    fn test_transcription_parser_committed() {
        let json = r#"{"text": "hello world", "confidence": 0.95, "message_type": "committed_transcript"}"#;
        let result = TranscriptionParser::parse_committed(json);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "hello world");
    }

    #[test]
    fn test_scribe_event_serialization() {
        let event = ScribeEvent::PartialTranscript {
            text: "test".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("partial_transcript"));
    }
}
