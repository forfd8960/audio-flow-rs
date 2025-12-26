//! 网络通信模块
//!
//! 提供 WebSocket 连接和 Scribe API 客户端

pub mod websocket;
pub mod scribe_client;

pub use websocket::{WebSocketClient, WebSocketConfig, ConnectionState, WsMessage, MessageBuilder};
pub use scribe_client::{ScribeClient, ScribeConfig, ScribeEvent, TranscriptionResult, TranscriptionParser};
