use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

/// 认证配置
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub token: Option<String>,
    pub api_keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: None,
            api_keys: Vec::new(),
        }
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(auth_config): State<Arc<AuthConfig>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 如果认证未启用，直接通过
    if !auth_config.enabled {
        debug!("Authentication disabled, allowing request");
        return Ok(next.run(request).await);
    }

    // 检查 Authorization 头
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    if let Some(auth_str) = auth_header {
        if auth_str.starts_with("Bearer ") {
            let token = &auth_str[7..];
            
            // 检查 token 是否有效
            if is_valid_token(&auth_config, token) {
                debug!("Valid token provided, allowing request");
                return Ok(next.run(request).await);
            }
        } else if auth_str.starts_with("ApiKey ") {
            let api_key = &auth_str[7..];
            
            // 检查 API key 是否有效
            if is_valid_api_key(&auth_config, api_key) {
                debug!("Valid API key provided, allowing request");
                return Ok(next.run(request).await);
            }
        }
    }

    warn!("Authentication failed for request");
    Err(StatusCode::UNAUTHORIZED)
}

/// 检查 token 是否有效
fn is_valid_token(auth_config: &AuthConfig, token: &str) -> bool {
    if let Some(ref valid_token) = auth_config.token {
        return token == valid_token;
    }
    false
}

/// 检查 API key 是否有效
fn is_valid_api_key(auth_config: &AuthConfig, api_key: &str) -> bool {
    auth_config.api_keys.contains(&api_key.to_string())
}

/// 认证响应
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authenticated: bool,
    pub message: String,
}

/// 生成认证 token
pub fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // 简单的 token 生成，实际应用中应该使用更安全的方法
    format!("cgminer_token_{}", timestamp)
}

/// 验证请求权限
pub fn verify_permissions(token: &str, required_permission: &str) -> bool {
    // 简化的权限验证，实际应用中应该有更复杂的权限系统
    debug!("Verifying permission '{}' for token", required_permission);
    
    // 暂时所有有效 token 都有所有权限
    !token.is_empty()
}
