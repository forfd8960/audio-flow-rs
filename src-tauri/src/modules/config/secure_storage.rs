//! 安全存储模块
//!
//! 使用系统安全存储 API 密钥（macOS Keychain, Windows Credential Manager）

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
#[auto_impl::auto_impl(&dyn)]
pub trait ApiKeyStorage {
    fn store(&self, service: &str, account: &str, key: &str) -> Result<(), SecureStorageError>;
    fn retrieve(&self, service: &str, account: &str) -> Result<Option<String>, SecureStorageError>;
    fn delete(&self, service: &str, account: &str) -> Result<(), SecureStorageError>;
}

/// macOS Keychain 实现
#[derive(Debug, Default)]
pub struct MacKeychainStorage;

impl MacKeychainStorage {
    pub fn new() -> Self {
        Self
    }
}

impl ApiKeyStorage for MacKeychainStorage {
    fn store(&self, service: &str, account: &str, key: &str) -> Result<(), SecureStorageError> {
        let output = std::process::Command::new("security")
            .args(&[
                "add-generic-password",
                "-a", account,
                "-s", service,
                "-w", key,
                "-U", // update if exists
            ])
            .output()
            .map_err(|e| SecureStorageError::StorageFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // 如果是已存在错误，仍然算成功（因为使用了 -U）
            if stderr.contains("The specified item already exists") {
                return Ok(());
            }
            return Err(SecureStorageError::StorageFailed(stderr.to_string()));
        }
        Ok(())
    }

    fn retrieve(&self, service: &str, account: &str) -> Result<Option<String>, SecureStorageError> {
        let output = std::process::Command::new("security")
            .args(&[
                "find-generic-password",
                "-a", account,
                "-s", service,
                "-w",
            ])
            .output()
            .map_err(|e| SecureStorageError::RetrievalFailed(e.to_string()))?;

        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if key.is_empty() {
                Ok(None)
            } else {
                Ok(Some(key))
            }
        } else {
            // Keychain 返回错误码 44 表示找不到项目
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("could not be found") || output.status.code() == Some(44) {
                Ok(None)
            } else {
                Err(SecureStorageError::RetrievalFailed(stderr.to_string()))
            }
        }
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), SecureStorageError> {
        let output = std::process::Command::new("security")
            .args(&[
                "delete-generic-password",
                "-a", account,
                "-s", service,
            ])
            .output()
            .map_err(|e| SecureStorageError::StorageFailed(e.to_string()))?;

        // 成功或"找不到项目"都算成功
        if output.status.success() || output.status.code() == Some(44) {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(SecureStorageError::StorageFailed(stderr.to_string()))
        }
    }
}

/// 跨平台安全存储
#[derive(Debug, Default)]
pub struct SecureStorage {
    storage: MacKeychainStorage,
}

impl SecureStorage {
    pub fn new() -> Self {
        Self {
            storage: MacKeychainStorage::new(),
        }
    }
}

impl ApiKeyStorage for SecureStorage {
    fn store(&self, service: &str, account: &str, key: &str) -> Result<(), SecureStorageError> {
        self.storage.store(service, account, key)
    }

    fn retrieve(&self, service: &str, account: &str) -> Result<Option<String>, SecureStorageError> {
        self.storage.retrieve(service, account)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), SecureStorageError> {
        self.storage.delete(service, account)
    }
}

/// ElevenLabs API 密钥存储
#[derive(Debug)]
pub struct ElevenLabsKeyStorage {
    storage: SecureStorage,
}

impl ElevenLabsKeyStorage {
    pub const SERVICE: &'static str = "audio-flow-elevenlabs";
    pub const ACCOUNT: &'static str = "api-key";

    pub fn new() -> Self {
        Self {
            storage: SecureStorage::new(),
        }
    }

    pub fn store_key(&self, key: &str) -> Result<(), SecureStorageError> {
        self.storage.store(Self::SERVICE, Self::ACCOUNT, key)
    }

    pub fn retrieve_key(&self) -> Result<Option<String>, SecureStorageError> {
        self.storage.retrieve(Self::SERVICE, Self::ACCOUNT)
    }

    pub fn delete_key(&self) -> Result<(), SecureStorageError> {
        self.storage.delete(Self::SERVICE, Self::ACCOUNT)
    }
}

impl Default for ElevenLabsKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevenlabs_storage_create() {
        let storage = ElevenLabsKeyStorage::new();
        assert_eq!(ElevenLabsKeyStorage::SERVICE, "audio-flow-elevenlabs");
        assert_eq!(ElevenLabsKeyStorage::ACCOUNT, "api-key");
    }
}
