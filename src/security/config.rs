//! 安全配置模块 - 简化版本
//!
//! 定义简化的安全相关配置结构和默认值

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 简化安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否启用安全功能
    pub enabled: bool,
    /// 配置文件保护
    pub config_protection: ConfigProtectionConfig,
    /// 数据加密（简化版本）
    pub data_encryption: SimpleDataEncryptionConfig,
    /// 自动备份
    pub backup: BackupConfig,
}

/// 配置文件保护配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProtectionConfig {
    /// 是否启用配置文件完整性检查
    pub enabled: bool,
    /// 配置文件路径列表
    pub config_paths: Vec<PathBuf>,
    /// 检查间隔（秒）
    pub check_interval: u64,
}

/// 简化数据加密配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleDataEncryptionConfig {
    /// 是否启用数据加密
    pub enabled: bool,
    /// 加密算法（固定为AES-256-GCM）
    pub algorithm: String,
}

/// 备份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// 是否启用自动备份
    pub enabled: bool,
    /// 备份目录
    pub backup_dir: PathBuf,
    /// 最大备份数量
    pub max_backups: usize,
    /// 备份间隔（秒）
    pub backup_interval: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            config_protection: ConfigProtectionConfig::default(),
            data_encryption: SimpleDataEncryptionConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

impl Default for ConfigProtectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            config_paths: vec![
                PathBuf::from("config.toml"),
                PathBuf::from("pools.toml"),
            ],
            check_interval: 300, // 5 minutes
        }
    }
}

impl Default for SimpleDataEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "AES-256-GCM".to_string(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backup_dir: PathBuf::from("backups"),
            max_backups: 10,
            backup_interval: 3600, // 1 hour
        }
    }
}

/// 从配置文件加载安全配置
pub fn load_security_config(config_path: &str) -> Result<SecurityConfig, Box<dyn std::error::Error>> {
    let config_content = std::fs::read_to_string(config_path)?;
    let config: SecurityConfig = toml::from_str(&config_content)?;
    Ok(config)
}

/// 保存安全配置到文件
pub fn save_security_config(config: &SecurityConfig, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_content = toml::to_string_pretty(config)?;
    std::fs::write(config_path, config_content)?;
    Ok(())
}

/// 验证简化安全配置
pub fn validate_security_config(config: &SecurityConfig) -> Result<(), String> {
    // 验证配置保护
    if config.config_protection.enabled {
        if config.config_protection.config_paths.is_empty() {
            return Err("配置保护启用时必须指定配置文件路径".to_string());
        }
    }

    // 验证备份配置
    if config.backup.enabled {
        if config.backup.max_backups == 0 {
            return Err("最大备份数量必须大于 0".to_string());
        }
    }

    Ok(())
}
