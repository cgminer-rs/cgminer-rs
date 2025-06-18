//! 简化安全模块
//!
//! 专为个人挖矿软件设计的轻量级安全功能
//! 只保留真正必要的安全保护，移除复杂的认证和权限管理

use crate::error::MiningError;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};
use rand::RngCore;

/// 简化安全管理器
pub struct SimpleSecurityManager {
    /// 配置保护器
    config_protector: ConfigProtector,
    /// 数据加密器
    data_encryptor: DataEncryptor,
    /// 备份管理器
    backup_manager: BackupManager,
    /// 是否启用
    enabled: bool,
}

/// 配置保护器
pub struct ConfigProtector {
    /// 配置文件路径
    config_paths: Vec<PathBuf>,
    /// 配置文件哈希值（用于完整性检查）
    config_hashes: HashMap<PathBuf, String>,
}

/// 数据加密器
pub struct DataEncryptor {
    /// 加密密钥
    cipher: Option<Aes256Gcm>,
    /// 是否启用加密
    enabled: bool,
}

/// 备份管理器
pub struct BackupManager {
    /// 备份目录
    backup_dir: PathBuf,
    /// 最大备份数量
    max_backups: usize,
}

/// 敏感数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedValue {
    /// 加密数据（Base64编码）
    pub data: String,
    /// 随机数（Base64编码）
    pub nonce: String,
}

/// 操作确认类型
#[derive(Debug, Clone)]
pub enum OperationType {
    /// 删除配置
    DeleteConfig,
    /// 停止挖矿
    StopMining,
    /// 修改钱包地址
    ChangeWallet,
    /// 重置设置
    ResetSettings,
}

impl SimpleSecurityManager {
    /// 创建新的简化安全管理器
    pub fn new(config_paths: Vec<PathBuf>, backup_dir: PathBuf) -> Result<Self, MiningError> {
        let config_protector = ConfigProtector::new(config_paths)?;
        let data_encryptor = DataEncryptor::new()?;
        let backup_manager = BackupManager::new(backup_dir)?;

        Ok(Self {
            config_protector,
            data_encryptor,
            backup_manager,
            enabled: true,
        })
    }

    /// 初始化安全系统
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        if !self.enabled {
            info!("🔓 安全功能已禁用");
            return Ok(());
        }

        info!("🔒 初始化简化安全系统");

        // 初始化配置保护
        self.config_protector.initialize().await?;
        info!("✅ 配置保护已启用");

        // 初始化数据加密
        self.data_encryptor.initialize().await?;
        info!("✅ 数据加密已启用");

        // 初始化备份管理
        self.backup_manager.initialize().await?;
        info!("✅ 自动备份已启用");

        info!("🔒 简化安全系统初始化完成");
        Ok(())
    }

    /// 检查配置文件完整性
    pub async fn check_config_integrity(&mut self) -> Result<bool, MiningError> {
        if !self.enabled {
            return Ok(true);
        }

        self.config_protector.check_integrity().await
    }

    /// 加密敏感数据
    pub async fn encrypt_sensitive_data(&self, data: &str) -> Result<String, MiningError> {
        if !self.enabled {
            return Ok(data.to_string());
        }

        let encrypted = self.data_encryptor.encrypt(data.as_bytes()).await?;
        serde_json::to_string(&encrypted)
            .map_err(|e| MiningError::security(format!("序列化失败: {}", e)))
    }

    /// 解密敏感数据
    pub async fn decrypt_sensitive_data(&self, encrypted_data: &str) -> Result<String, MiningError> {
        if !self.enabled {
            return Ok(encrypted_data.to_string());
        }

        let encrypted: EncryptedValue = serde_json::from_str(encrypted_data)
            .map_err(|e| MiningError::security(format!("反序列化失败: {}", e)))?;
        let decrypted = self.data_encryptor.decrypt(&encrypted).await?;
        String::from_utf8(decrypted)
            .map_err(|e| MiningError::security(format!("UTF-8解码失败: {}", e)))
    }

    /// 创建配置备份
    pub async fn backup_config(&self, config_path: &Path) -> Result<PathBuf, MiningError> {
        if !self.enabled {
            return Err(MiningError::security("安全功能未启用".to_string()));
        }

        self.backup_manager.create_backup(config_path).await
    }

    /// 恢复配置备份
    pub async fn restore_config(&self, config_path: &Path) -> Result<(), MiningError> {
        if !self.enabled {
            return Err(MiningError::security("安全功能未启用".to_string()));
        }

        self.backup_manager.restore_latest_backup(config_path).await
    }

    /// 请求操作确认
    pub fn request_confirmation(&self, operation: OperationType) -> bool {
        if !self.enabled {
            return true;
        }

        let message = match operation {
            OperationType::DeleteConfig => "⚠️  确认删除配置文件？这将无法撤销！",
            OperationType::StopMining => "⚠️  确认停止挖矿？",
            OperationType::ChangeWallet => "⚠️  确认修改钱包地址？请确保新地址正确！",
            OperationType::ResetSettings => "⚠️  确认重置所有设置？这将恢复默认配置！",
        };

        warn!("{}", message);
        // 在实际应用中，这里应该显示确认对话框
        // 现在简化为总是返回 true，实际使用时可以集成用户界面
        true
    }

    /// 启用/禁用安全功能
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            info!("🔒 安全功能已启用");
        } else {
            info!("🔓 安全功能已禁用");
        }
    }

    /// 获取安全状态
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl ConfigProtector {
    /// 创建配置保护器
    pub fn new(config_paths: Vec<PathBuf>) -> Result<Self, MiningError> {
        Ok(Self {
            config_paths,
            config_hashes: HashMap::new(),
        })
    }

    /// 初始化配置保护
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        // 计算所有配置文件的哈希值
        for path in &self.config_paths {
            if path.exists() {
                let hash = self.calculate_file_hash(path)?;
                self.config_hashes.insert(path.clone(), hash);
                debug!("配置文件哈希已记录: {:?}", path);
            }
        }
        Ok(())
    }

    /// 检查配置完整性
    pub async fn check_integrity(&mut self) -> Result<bool, MiningError> {
        let mut all_valid = true;

        for path in &self.config_paths {
            if path.exists() {
                let current_hash = self.calculate_file_hash(path)?;

                if let Some(stored_hash) = self.config_hashes.get(path) {
                    if current_hash != *stored_hash {
                        warn!("⚠️  配置文件可能被修改: {:?}", path);
                        all_valid = false;
                        // 更新哈希值
                        self.config_hashes.insert(path.clone(), current_hash);
                    }
                } else {
                    // 新文件，记录哈希值
                    self.config_hashes.insert(path.clone(), current_hash);
                }
            }
        }

        Ok(all_valid)
    }

    /// 计算文件哈希值
    fn calculate_file_hash(&self, path: &Path) -> Result<String, MiningError> {
        use sha2::{Sha256, Digest};

        let content = fs::read(path)
            .map_err(|e| MiningError::security(format!("读取配置文件失败: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }
}

impl DataEncryptor {
    /// 创建数据加密器
    pub fn new() -> Result<Self, MiningError> {
        Ok(Self {
            cipher: None,
            enabled: true,
        })
    }

    /// 初始化加密器
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        if self.enabled {
            // 生成简单的加密密钥
            let key = Aes256Gcm::generate_key(&mut OsRng);
            self.cipher = Some(Aes256Gcm::new(&key));
            debug!("数据加密器已初始化");
        }
        Ok(())
    }

    /// 加密数据
    pub async fn encrypt(&self, data: &[u8]) -> Result<EncryptedValue, MiningError> {
        if !self.enabled {
            return Err(MiningError::security("加密功能未启用".to_string()));
        }

        let cipher = self.cipher.as_ref()
            .ok_or_else(|| MiningError::security("加密器未初始化".to_string()))?;

        // 生成随机数
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| MiningError::security(format!("数据加密失败: {}", e)))?;

        Ok(EncryptedValue {
            data: general_purpose::STANDARD.encode(&ciphertext),
            nonce: general_purpose::STANDARD.encode(&nonce_bytes),
        })
    }

    /// 解密数据
    pub async fn decrypt(&self, encrypted: &EncryptedValue) -> Result<Vec<u8>, MiningError> {
        if !self.enabled {
            return Err(MiningError::security("加密功能未启用".to_string()));
        }

        let cipher = self.cipher.as_ref()
            .ok_or_else(|| MiningError::security("加密器未初始化".to_string()))?;

        // 解码数据
        let ciphertext = general_purpose::STANDARD.decode(&encrypted.data)
            .map_err(|e| MiningError::security(format!("密文解码失败: {}", e)))?;

        let nonce_bytes = general_purpose::STANDARD.decode(&encrypted.nonce)
            .map_err(|e| MiningError::security(format!("随机数解码失败: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // 解密数据
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| MiningError::security(format!("数据解密失败: {}", e)))?;

        Ok(plaintext)
    }
}

impl BackupManager {
    /// 创建备份管理器
    pub fn new(backup_dir: PathBuf) -> Result<Self, MiningError> {
        Ok(Self {
            backup_dir,
            max_backups: 5, // 保留最近5个备份
        })
    }

    /// 初始化备份管理器
    pub async fn initialize(&self) -> Result<(), MiningError> {
        // 创建备份目录
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir)
                .map_err(|e| MiningError::security(format!("创建备份目录失败: {}", e)))?;
        }
        debug!("备份目录已准备: {:?}", self.backup_dir);
        Ok(())
    }

    /// 创建备份
    pub async fn create_backup(&self, config_path: &Path) -> Result<PathBuf, MiningError> {
        if !config_path.exists() {
            return Err(MiningError::security("配置文件不存在".to_string()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let file_name = config_path.file_name()
            .ok_or_else(|| MiningError::security("无效的文件名".to_string()))?;

        let backup_name = format!("{}_{}.backup",
            file_name.to_string_lossy(),
            timestamp);

        let backup_path = self.backup_dir.join(backup_name);

        fs::copy(config_path, &backup_path)
            .map_err(|e| MiningError::security(format!("创建备份失败: {}", e)))?;

        // 清理旧备份
        self.cleanup_old_backups(config_path).await?;

        info!("✅ 配置备份已创建: {:?}", backup_path);
        Ok(backup_path)
    }

    /// 恢复最新备份
    pub async fn restore_latest_backup(&self, config_path: &Path) -> Result<(), MiningError> {
        let latest_backup = self.find_latest_backup(config_path).await?;

        fs::copy(&latest_backup, config_path)
            .map_err(|e| MiningError::security(format!("恢复备份失败: {}", e)))?;

        info!("✅ 配置已从备份恢复: {:?}", latest_backup);
        Ok(())
    }

    /// 查找最新备份
    async fn find_latest_backup(&self, config_path: &Path) -> Result<PathBuf, MiningError> {
        let file_name = config_path.file_name()
            .ok_or_else(|| MiningError::security("无效的文件名".to_string()))?
            .to_string_lossy();

        let mut backups = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&*file_name) && name.ends_with(".backup") {
                        backups.push(path);
                    }
                }
            }
        }

        if backups.is_empty() {
            return Err(MiningError::security("没有找到备份文件".to_string()));
        }

        // 按修改时间排序，返回最新的
        backups.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
            let b_time = b.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        Ok(backups[0].clone())
    }

    /// 清理旧备份
    async fn cleanup_old_backups(&self, config_path: &Path) -> Result<(), MiningError> {
        let file_name = config_path.file_name()
            .ok_or_else(|| MiningError::security("无效的文件名".to_string()))?
            .to_string_lossy();

        let mut backups = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&*file_name) && name.ends_with(".backup") {
                        if let Ok(metadata) = entry.metadata() {
                            backups.push((path, metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }

        // 按时间排序
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        // 删除超过限制的备份
        if backups.len() > self.max_backups {
            for (path, _) in backups.iter().skip(self.max_backups) {
                if let Err(e) = fs::remove_file(path) {
                    warn!("删除旧备份失败: {} - {}", path.display(), e);
                } else {
                    debug!("删除旧备份: {}", path.display());
                }
            }
        }

        Ok(())
    }
}
