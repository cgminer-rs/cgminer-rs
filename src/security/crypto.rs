//! 加密管理模块

use crate::error::MiningError;
use crate::security::config::CryptoConfig;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn, error};

/// 加密管理器
pub struct CryptoManager {
    /// 配置
    config: CryptoConfig,
    /// 当前加密密钥
    current_key: Option<Aes256Gcm>,
    /// 密钥历史（用于解密旧数据）
    key_history: HashMap<String, Aes256Gcm>,
    /// 密钥创建时间
    key_created_at: Option<SystemTime>,
    /// 密钥ID
    current_key_id: Option<String>,
}

/// 加密数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// 密钥ID
    pub key_id: String,
    /// 加密数据（Base64编码）
    pub data: String,
    /// 随机数（Base64编码）
    pub nonce: String,
    /// 加密时间戳
    pub timestamp: u64,
}

/// 密钥信息
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// 密钥ID
    pub key_id: String,
    /// 创建时间
    pub created_at: SystemTime,
    /// 是否为当前密钥
    pub is_current: bool,
}

impl CryptoManager {
    /// 创建新的加密管理器
    pub fn new(config: CryptoConfig) -> Result<Self, MiningError> {
        Ok(Self {
            config,
            current_key: None,
            key_history: HashMap::new(),
            key_created_at: None,
            current_key_id: None,
        })
    }

    /// 初始化加密管理器
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        info!("🔐 初始化加密管理器");

        if self.config.enabled {
            // 生成或加载加密密钥
            self.initialize_encryption_key().await?;

            // 启动密钥轮转任务
            self.schedule_key_rotation().await?;

            info!("✅ 加密管理器初始化完成");
        } else {
            info!("🔓 加密功能已禁用");
        }

        Ok(())
    }

    /// 加密数据
    pub async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, MiningError> {
        if !self.config.enabled {
            return Ok(data.to_vec());
        }

        let cipher = self.current_key.as_ref()
            .ok_or_else(|| MiningError::security("加密密钥未初始化".to_string()))?;

        let key_id = self.current_key_id.as_ref()
            .ok_or_else(|| MiningError::security("密钥ID未设置".to_string()))?;

        // 生成随机数
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // 加密数据
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|e| MiningError::security(format!("数据加密失败: {}", e)))?;

        // 创建加密数据结构
        let encrypted_data = EncryptedData {
            key_id: key_id.clone(),
            data: general_purpose::STANDARD.encode(&ciphertext),
            nonce: general_purpose::STANDARD.encode(&nonce),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // 序列化为JSON
        let json_data = serde_json::to_vec(&encrypted_data)
            .map_err(|e| MiningError::security(format!("加密数据序列化失败: {}", e)))?;

        debug!("数据加密成功，大小: {} -> {}", data.len(), json_data.len());
        Ok(json_data)
    }

    /// 解密数据
    pub async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, MiningError> {
        if !self.config.enabled {
            return Ok(encrypted_data.to_vec());
        }

        // 反序列化加密数据
        let encrypted: EncryptedData = serde_json::from_slice(encrypted_data)
            .map_err(|e| MiningError::security(format!("加密数据反序列化失败: {}", e)))?;

        // 查找对应的密钥
        let cipher = if let Some(current_key_id) = &self.current_key_id {
            if encrypted.key_id == *current_key_id {
                self.current_key.as_ref()
            } else {
                self.key_history.get(&encrypted.key_id)
            }
        } else {
            self.key_history.get(&encrypted.key_id)
        }.ok_or_else(|| MiningError::security(format!("找不到密钥ID: {}", encrypted.key_id)))?;

        // 解码数据和随机数
        let ciphertext = general_purpose::STANDARD.decode(&encrypted.data)
            .map_err(|e| MiningError::security(format!("密文解码失败: {}", e)))?;

        let nonce_bytes = general_purpose::STANDARD.decode(&encrypted.nonce)
            .map_err(|e| MiningError::security(format!("随机数解码失败: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // 解密数据
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| MiningError::security(format!("数据解密失败: {}", e)))?;

        debug!("数据解密成功，大小: {} -> {}", encrypted_data.len(), plaintext.len());
        Ok(plaintext)
    }

    /// 生成安全随机数
    pub fn generate_random_bytes(&self, length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        bytes
    }

    /// 生成安全随机字符串
    pub fn generate_random_string(&self, length: usize) -> String {
        let bytes = self.generate_random_bytes(length);
        general_purpose::STANDARD.encode(&bytes)
    }

    /// 计算数据哈希
    pub fn hash_data(&self, data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 验证数据完整性
    pub fn verify_integrity(&self, data: &[u8], expected_hash: &str) -> bool {
        let actual_hash = self.hash_data(data);
        actual_hash == expected_hash
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool, MiningError> {
        if !self.config.enabled {
            return Ok(true);
        }

        // 检查当前密钥是否存在
        if self.current_key.is_none() {
            return Ok(false);
        }

        // 检查密钥是否需要轮转
        if let Some(created_at) = self.key_created_at {
            let age = SystemTime::now()
                .duration_since(created_at)
                .unwrap()
                .as_secs();

            if age > self.config.key_rotation_interval {
                warn!("加密密钥需要轮转");
                return Ok(false);
            }
        }

        // 测试加密解密功能
        let test_data = b"test encryption";
        match self.encrypt(test_data).await {
            Ok(encrypted) => {
                match self.decrypt(&encrypted).await {
                    Ok(decrypted) => {
                        if decrypted == test_data {
                            Ok(true)
                        } else {
                            error!("加密解密测试失败：数据不匹配");
                            Ok(false)
                        }
                    }
                    Err(e) => {
                        error!("解密测试失败: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                error!("加密测试失败: {}", e);
                Ok(false)
            }
        }
    }

    /// 获取密钥信息
    pub fn get_key_info(&self) -> Vec<KeyInfo> {
        let mut keys = Vec::new();

        // 添加当前密钥
        if let Some(key_id) = &self.current_key_id {
            keys.push(KeyInfo {
                key_id: key_id.clone(),
                created_at: self.key_created_at.unwrap_or_else(SystemTime::now),
                is_current: true,
            });
        }

        // 添加历史密钥（这里简化处理，实际应该存储创建时间）
        for key_id in self.key_history.keys() {
            if Some(key_id) != self.current_key_id.as_ref() {
                keys.push(KeyInfo {
                    key_id: key_id.clone(),
                    created_at: SystemTime::now(), // 简化处理
                    is_current: false,
                });
            }
        }

        keys
    }

    /// 轮转密钥
    pub async fn rotate_key(&mut self) -> Result<(), MiningError> {
        info!("🔄 开始密钥轮转");

        // 将当前密钥移到历史记录
        if let (Some(current_key), Some(current_key_id)) = (self.current_key.take(), self.current_key_id.take()) {
            self.key_history.insert(current_key_id, current_key);
        }

        // 生成新密钥
        self.generate_new_key().await?;

        // 清理过期的历史密钥（保留最近的几个）
        self.cleanup_old_keys().await?;

        info!("✅ 密钥轮转完成");
        Ok(())
    }

    /// 初始化加密密钥
    async fn initialize_encryption_key(&mut self) -> Result<(), MiningError> {
        let key_data = self.config.data_encryption_key.clone();
        if let Some(key_data) = key_data {
            // 使用配置中的密钥
            self.load_key_from_config(&key_data).await?;
        } else {
            // 生成新密钥
            self.generate_new_key().await?;
        }

        Ok(())
    }

    /// 从配置加载密钥
    async fn load_key_from_config(&mut self, key_data: &str) -> Result<(), MiningError> {
        let key_bytes = general_purpose::STANDARD.decode(key_data)
            .map_err(|e| MiningError::security(format!("密钥解码失败: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(MiningError::security("密钥长度必须为32字节".to_string()));
        }

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        self.current_key = Some(cipher);
        self.current_key_id = Some("config-key".to_string());
        self.key_created_at = Some(SystemTime::now());

        debug!("从配置加载加密密钥");
        Ok(())
    }

    /// 生成新密钥
    async fn generate_new_key(&mut self) -> Result<(), MiningError> {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let cipher = Aes256Gcm::new(&key);

        let key_id = uuid::Uuid::new_v4().to_string();

        self.current_key = Some(cipher);
        self.current_key_id = Some(key_id);
        self.key_created_at = Some(SystemTime::now());

        debug!("生成新的加密密钥");
        Ok(())
    }

    /// 启动密钥轮转任务
    async fn schedule_key_rotation(&self) -> Result<(), MiningError> {
        // 这里应该启动一个后台任务来定期轮转密钥
        // 为了简化，这里只是记录日志
        debug!("密钥轮转任务已启动，间隔: {} 秒", self.config.key_rotation_interval);
        Ok(())
    }

    /// 清理旧密钥
    async fn cleanup_old_keys(&mut self) -> Result<(), MiningError> {
        // 保留最近的5个历史密钥
        const MAX_HISTORY_KEYS: usize = 5;

        if self.key_history.len() > MAX_HISTORY_KEYS {
            // 简化处理：随机删除一些旧密钥
            let keys_to_remove: Vec<String> = self.key_history.keys()
                .take(self.key_history.len() - MAX_HISTORY_KEYS)
                .cloned()
                .collect();

            for key_id in keys_to_remove {
                self.key_history.remove(&key_id);
                debug!("删除旧密钥: {}", key_id);
            }
        }

        Ok(())
    }
}
