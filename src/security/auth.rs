//! 认证管理模块

use crate::error::MiningError;
use crate::security::config::AuthConfig;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 认证管理器
pub struct AuthManager {
    /// 配置
    config: AuthConfig,
    /// 用户存储
    users: HashMap<String, User>,
    /// 活跃会话
    active_sessions: HashMap<String, Session>,
    /// 登录尝试记录
    login_attempts: HashMap<String, LoginAttempts>,
}

/// 用户信息
#[derive(Debug, Clone)]
pub struct User {
    /// 用户ID
    pub id: String,
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
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后登录时间
    pub last_login: Option<SystemTime>,
}

/// 会话信息
#[derive(Debug, Clone)]
pub struct Session {
    /// 会话ID
    pub session_id: String,
    /// 用户ID
    pub user_id: String,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后活跃时间
    pub last_active: SystemTime,
    /// 过期时间
    pub expires_at: SystemTime,
}

/// 登录尝试记录
#[derive(Debug, Clone)]
pub struct LoginAttempts {
    /// 尝试次数
    pub count: u32,
    /// 最后尝试时间
    pub last_attempt: SystemTime,
    /// 是否被锁定
    pub locked: bool,
    /// 锁定到期时间
    pub locked_until: Option<SystemTime>,
}

/// 认证令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// 用户ID
    pub user_id: String,
    /// 用户名
    pub username: String,
    /// 角色
    pub role: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 过期时间
    pub exp: u64,
    /// 签发时间
    pub iat: u64,
    /// JWT ID
    pub jti: String,
}

/// JWT 声明
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// 用户ID
    sub: String,
    /// 用户名
    username: String,
    /// 角色
    role: String,
    /// 权限
    permissions: Vec<String>,
    /// 过期时间
    exp: u64,
    /// 签发时间
    iat: u64,
    /// JWT ID
    jti: String,
}

/// 认证结果
#[derive(Debug)]
pub struct AuthResult {
    /// 认证令牌
    pub token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 用户信息
    pub user: User,
}

impl AuthManager {
    /// 创建新的认证管理器
    pub fn new(config: AuthConfig) -> Result<Self, MiningError> {
        let mut manager = Self {
            config,
            users: HashMap::new(),
            active_sessions: HashMap::new(),
            login_attempts: HashMap::new(),
        };

        // 初始化默认用户
        manager.initialize_default_users()?;

        Ok(manager)
    }

    /// 初始化认证管理器
    pub async fn initialize(&mut self) -> Result<(), MiningError> {
        info!("🔐 初始化认证管理器");

        // 清理过期会话
        self.cleanup_expired_sessions().await?;

        // 重置登录尝试记录
        self.reset_expired_lockouts().await?;

        info!("✅ 认证管理器初始化完成");
        Ok(())
    }

    /// 用户认证
    pub async fn authenticate(&mut self, username: &str, password: &str) -> Result<AuthToken, MiningError> {
        debug!("尝试认证用户: {}", username);

        // 检查登录尝试限制
        if self.is_user_locked(username).await? {
            warn!("用户 {} 因多次登录失败被锁定", username);
            return Err(MiningError::security("用户被锁定，请稍后再试".to_string()));
        }

        // 查找用户并克隆必要信息
        let (user_id, password_hash, user_clone) = {
            let user = self.users.get(username)
                .ok_or_else(|| MiningError::security("用户名或密码错误".to_string()))?;

            if !user.enabled {
                warn!("用户 {} 已被禁用", username);
                return Err(MiningError::security("用户已被禁用".to_string()));
            }

            (user.id.clone(), user.password_hash.clone(), user.clone())
        };

        // 验证密码
        if !self.verify_password(password, &password_hash)? {
            self.record_failed_login(username).await?;
            warn!("用户 {} 密码验证失败", username);
            return Err(MiningError::security("用户名或密码错误".to_string()));
        }

        // 重置登录尝试记录
        self.reset_login_attempts(username).await?;

        // 生成认证令牌
        let token = self.generate_token(&user_clone)?;

        // 创建会话
        self.create_session(&user_id).await?;

        // 更新最后登录时间
        if let Some(user) = self.users.get_mut(username) {
            user.last_login = Some(SystemTime::now());
        }

        info!("用户 {} 认证成功", username);
        Ok(token)
    }

    /// 验证令牌
    pub async fn verify_token(&self, token: &str) -> Result<AuthToken, MiningError> {
        let decoding_key = DecodingKey::from_secret(self.config.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| MiningError::security(format!("令牌验证失败: {}", e)))?;

        let claims = token_data.claims;

        // 检查用户是否仍然存在且启用
        let user = self.users.get(&claims.username)
            .ok_or_else(|| MiningError::security("用户不存在".to_string()))?;

        if !user.enabled {
            return Err(MiningError::security("用户已被禁用".to_string()));
        }

        Ok(AuthToken {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            permissions: claims.permissions,
            exp: claims.exp,
            iat: claims.iat,
            jti: claims.jti,
        })
    }

    /// 授权检查
    pub async fn authorize(&self, token: &AuthToken, resource: &str, action: &str) -> Result<bool, MiningError> {
        debug!("检查用户 {} 对资源 {} 的 {} 权限", token.username, resource, action);

        // 检查令牌是否过期
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if token.exp < now {
            return Err(MiningError::security("令牌已过期".to_string()));
        }

        // 构造权限字符串
        let required_permission = format!("{}:{}", resource, action);

        // 检查用户权限
        let wildcard_resource = format!("{}:*", resource);
        let wildcard_all = "*:*".to_string();
        let has_permission = token.permissions.contains(&required_permission) ||
                           token.permissions.contains(&wildcard_resource) ||
                           token.permissions.contains(&wildcard_all);

        debug!("权限检查结果: {}", has_permission);
        Ok(has_permission)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool, MiningError> {
        // 检查配置是否有效
        if self.config.jwt_secret.len() < 32 {
            return Ok(false);
        }

        // 检查是否有活跃用户
        let active_users = self.users.values().filter(|u| u.enabled).count();
        if active_users == 0 {
            return Ok(false);
        }

        Ok(true)
    }

    /// 初始化默认用户
    fn initialize_default_users(&mut self) -> Result<(), MiningError> {
        for user_config in &self.config.default_users {
            let user = User {
                id: Uuid::new_v4().to_string(),
                username: user_config.username.clone(),
                password_hash: user_config.password_hash.clone(),
                role: user_config.role.clone(),
                permissions: user_config.permissions.clone(),
                enabled: user_config.enabled,
                created_at: SystemTime::now(),
                last_login: None,
            };

            self.users.insert(user_config.username.clone(), user);
        }

        Ok(())
    }

    /// 生成认证令牌
    fn generate_token(&self, user: &User) -> Result<AuthToken, MiningError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let exp = now + self.config.token_expiry;
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            permissions: user.permissions.clone(),
            exp,
            iat: now,
            jti: jti.clone(),
        };

        let encoding_key = EncodingKey::from_secret(self.config.jwt_secret.as_ref());
        let _token = encode(&Header::default(), &claims, &encoding_key)
            .map_err(|e| MiningError::security(format!("令牌生成失败: {}", e)))?;

        Ok(AuthToken {
            user_id: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            permissions: user.permissions.clone(),
            exp,
            iat: now,
            jti,
        })
    }

    /// 验证密码
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, MiningError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| MiningError::security(format!("密码哈希解析失败: {}", e)))?;

        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// 哈希密码
    pub fn hash_password(&self, password: &str) -> Result<String, MiningError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| MiningError::security(format!("密码哈希失败: {}", e)))?;

        Ok(password_hash.to_string())
    }

    /// 检查用户是否被锁定
    async fn is_user_locked(&self, username: &str) -> Result<bool, MiningError> {
        if let Some(attempts) = self.login_attempts.get(username) {
            if attempts.locked {
                if let Some(locked_until) = attempts.locked_until {
                    return Ok(SystemTime::now() < locked_until);
                }
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 记录登录失败
    async fn record_failed_login(&mut self, username: &str) -> Result<(), MiningError> {
        let now = SystemTime::now();
        let attempts = self.login_attempts.entry(username.to_string()).or_insert(LoginAttempts {
            count: 0,
            last_attempt: now,
            locked: false,
            locked_until: None,
        });

        attempts.count += 1;
        attempts.last_attempt = now;

        if attempts.count >= self.config.max_login_attempts {
            attempts.locked = true;
            attempts.locked_until = Some(now + std::time::Duration::from_secs(self.config.lockout_duration));
            warn!("用户 {} 因多次登录失败被锁定", username);
        }

        Ok(())
    }

    /// 重置登录尝试记录
    async fn reset_login_attempts(&mut self, username: &str) -> Result<(), MiningError> {
        self.login_attempts.remove(username);
        Ok(())
    }

    /// 创建会话
    async fn create_session(&mut self, user_id: &str) -> Result<String, MiningError> {
        let session_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();

        let session = Session {
            session_id: session_id.clone(),
            user_id: user_id.to_string(),
            created_at: now,
            last_active: now,
            expires_at: now + std::time::Duration::from_secs(self.config.session_timeout),
        };

        self.active_sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// 清理过期会话
    async fn cleanup_expired_sessions(&mut self) -> Result<(), MiningError> {
        let now = SystemTime::now();
        self.active_sessions.retain(|_, session| session.expires_at > now);
        Ok(())
    }

    /// 重置过期的锁定
    async fn reset_expired_lockouts(&mut self) -> Result<(), MiningError> {
        let now = SystemTime::now();
        for attempts in self.login_attempts.values_mut() {
            if attempts.locked {
                if let Some(locked_until) = attempts.locked_until {
                    if now >= locked_until {
                        attempts.locked = false;
                        attempts.locked_until = None;
                        attempts.count = 0;
                    }
                }
            }
        }
        Ok(())
    }
}
