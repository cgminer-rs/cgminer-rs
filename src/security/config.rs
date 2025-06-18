//! 安全配置模块

use serde::{Deserialize, Serialize};
use std::collections::HashMap;


/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 认证配置
    pub auth_config: AuthConfig,
    /// 加密配置
    pub crypto_config: CryptoConfig,
    /// 审计配置
    pub audit_config: AuditConfig,
    /// 安全策略配置
    pub policy_config: PolicyConfig,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 是否启用认证
    pub enabled: bool,
    /// JWT 密钥
    pub jwt_secret: String,
    /// Token 过期时间（秒）
    pub token_expiry: u64,
    /// 刷新 Token 过期时间（秒）
    pub refresh_token_expiry: u64,
    /// API 密钥列表
    pub api_keys: Vec<String>,
    /// 默认用户配置
    pub default_users: Vec<UserConfig>,
    /// 会话超时时间（秒）
    pub session_timeout: u64,
    /// 最大登录尝试次数
    pub max_login_attempts: u32,
    /// 登录锁定时间（秒）
    pub lockout_duration: u64,
}

/// 用户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// 用户名
    pub username: String,
    /// 密码哈希
    pub password_hash: String,
    /// 角色
    pub role: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 加密配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    /// 是否启用加密
    pub enabled: bool,
    /// 加密算法
    pub algorithm: String,
    /// 密钥长度
    pub key_length: usize,
    /// 密钥轮转间隔（秒）
    pub key_rotation_interval: u64,
    /// 数据加密密钥
    pub data_encryption_key: Option<String>,
    /// 传输加密配置
    pub transport_encryption: TransportEncryptionConfig,
}

/// 传输加密配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportEncryptionConfig {
    /// 是否启用 TLS
    pub tls_enabled: bool,
    /// TLS 证书路径
    pub cert_path: Option<String>,
    /// TLS 私钥路径
    pub key_path: Option<String>,
    /// 最小 TLS 版本
    pub min_tls_version: String,
}

/// 审计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// 是否启用审计
    pub enabled: bool,
    /// 审计日志路径
    pub log_path: String,
    /// 日志轮转大小（字节）
    pub max_log_size: u64,
    /// 保留日志文件数量
    pub max_log_files: u32,
    /// 审计事件过滤器
    pub event_filters: Vec<String>,
    /// 是否记录敏感数据
    pub log_sensitive_data: bool,
    /// 日志格式
    pub log_format: String,
}

/// 安全策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// 密码策略
    pub password_policy: PasswordPolicy,
    /// 访问控制策略
    pub access_control: AccessControlPolicy,
    /// 速率限制策略
    pub rate_limiting: RateLimitingPolicy,
}

/// 密码策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    /// 最小长度
    pub min_length: usize,
    /// 是否需要大写字母
    pub require_uppercase: bool,
    /// 是否需要小写字母
    pub require_lowercase: bool,
    /// 是否需要数字
    pub require_numbers: bool,
    /// 是否需要特殊字符
    pub require_special_chars: bool,
    /// 密码过期时间（天）
    pub expiry_days: u32,
    /// 密码历史记录数量
    pub history_count: u32,
}

/// 访问控制策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlPolicy {
    /// 默认权限
    pub default_permissions: Vec<String>,
    /// 角色权限映射
    pub role_permissions: HashMap<String, Vec<String>>,
    /// IP 白名单
    pub ip_whitelist: Vec<String>,
    /// IP 黑名单
    pub ip_blacklist: Vec<String>,
}

/// 速率限制策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingPolicy {
    /// 是否启用速率限制
    pub enabled: bool,
    /// 每分钟最大请求数
    pub requests_per_minute: u32,
    /// 每小时最大请求数
    pub requests_per_hour: u32,
    /// 突发请求限制
    pub burst_limit: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_config: AuthConfig::default(),
            crypto_config: CryptoConfig::default(),
            audit_config: AuditConfig::default(),
            policy_config: PolicyConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: "default-jwt-secret-change-in-production".to_string(),
            token_expiry: 3600, // 1 hour
            refresh_token_expiry: 86400 * 7, // 7 days
            api_keys: vec![],
            default_users: vec![],
            session_timeout: 1800, // 30 minutes
            max_login_attempts: 5,
            lockout_duration: 900, // 15 minutes
        }
    }
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "AES-256-GCM".to_string(),
            key_length: 256,
            key_rotation_interval: 86400 * 30, // 30 days
            data_encryption_key: None,
            transport_encryption: TransportEncryptionConfig::default(),
        }
    }
}

impl Default for TransportEncryptionConfig {
    fn default() -> Self {
        Self {
            tls_enabled: false,
            cert_path: None,
            key_path: None,
            min_tls_version: "1.2".to_string(),
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_path: "logs/audit.log".to_string(),
            max_log_size: 100 * 1024 * 1024, // 100MB
            max_log_files: 10,
            event_filters: vec![],
            log_sensitive_data: false,
            log_format: "json".to_string(),
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            password_policy: PasswordPolicy::default(),
            access_control: AccessControlPolicy::default(),
            rate_limiting: RateLimitingPolicy::default(),
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special_chars: false,
            expiry_days: 90,
            history_count: 5,
        }
    }
}

impl Default for AccessControlPolicy {
    fn default() -> Self {
        Self {
            default_permissions: vec!["read".to_string()],
            role_permissions: HashMap::new(),
            ip_whitelist: vec![],
            ip_blacklist: vec![],
        }
    }
}

impl Default for RateLimitingPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_limit: 10,
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

/// 验证安全配置
pub fn validate_security_config(config: &SecurityConfig) -> Result<(), String> {
    // 验证认证配置
    if config.auth_config.enabled {
        if config.auth_config.jwt_secret.len() < 32 {
            return Err("JWT 密钥长度至少需要 32 个字符".to_string());
        }

        if config.auth_config.token_expiry == 0 {
            return Err("Token 过期时间必须大于 0".to_string());
        }
    }

    // 验证加密配置
    if config.crypto_config.enabled {
        if config.crypto_config.key_length < 128 {
            return Err("加密密钥长度至少需要 128 位".to_string());
        }
    }

    // 验证审计配置
    if config.audit_config.enabled {
        if config.audit_config.log_path.is_empty() {
            return Err("审计日志路径不能为空".to_string());
        }
    }

    Ok(())
}
