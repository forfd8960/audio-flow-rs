//! 安全存储模块

use thiserror::Error;

/// 安全存储错误
#[derive(Debug, Error)]
pub enum SecureStorageError {
    #[error("Storage failed: {0}")]
    StorageFailed(String),
    #[error("Retrieval failed: {0}")]
    RetrievalFailed(String),
    #[error("Key not found")]
    NotFound,
}

/// API 密钥存储 trait
pub trait ApiKeyStorage {
    fn store(&self, key: &str) -> Result<(), SecureStorageError>;
    fn retrieve(&self) -> Result<Option<String>, SecureStorageError>;
    fn delete(&self) -> Result<(), SecureStorageError>;
}

/// 跨平台安全存储
#[derive(Debug)]
pub struct SecureStorage;

impl SecureStorage {
    pub fn new() -> Self {
        Self
    }
}

impl ApiKeyStorage for SecureStorage {
    fn store(&self, _key: &str) -> Result<(), SecureStorageError> {
        Ok(())
    }

    fn retrieve(&self) -> Result<Option<String>, SecureStorageError> {
        Ok(None)
    }

    fn delete(&self) -> Result<(), SecureStorageError> {
        Ok(())
    }
}
