use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscordRpcError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Handshake error: {0}")]
    HandshakeError(String),

    #[error("Not connected to Discord")]
    NotConnected,

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    #[error("Heartbeat failed: {0}")]
    HeartbeatFailed(String),
}
