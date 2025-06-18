use crate::api::{AppState, create_routes};
use crate::config::ApiConfig;
use crate::error::ApiError;
use crate::mining::MiningManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, Any},
    trace::TraceLayer,
    timeout::TimeoutLayer,
};
use tracing::{info, warn, error};
use std::time::Duration;

/// API 服务器
pub struct ApiServer {
    /// 服务器配置
    config: ApiConfig,
    /// 挖矿管理器
    mining_manager: Arc<MiningManager>,
    /// 服务器句柄
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl ApiServer {
    /// 创建新的 API 服务器
    pub fn new(config: ApiConfig, mining_manager: Arc<MiningManager>) -> Self {
        Self {
            config,
            mining_manager,
            server_handle: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动 API 服务器
    pub async fn start(&self) -> Result<(), ApiError> {
        if !self.config.enabled {
            info!("API server is disabled");
            return Ok(());
        }

        info!("Starting API server on {}:{}", self.config.bind_address, self.config.port);

        // 检查是否已经在运行
        if *self.running.read().await {
            warn!("API server is already running");
            return Ok(());
        }

        // 创建应用状态
        let app_state = AppState {
            mining_manager: self.mining_manager.clone(),
        };

        // 创建路由
        let app = create_routes(app_state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(self.create_cors_layer())
            );

        // 解析绑定地址
        let addr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| ApiError::ServerStartFailed {
                error: format!("Invalid bind address: {}", e),
            })?;

        // 启动服务器
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| ApiError::ServerStartFailed {
                error: format!("Failed to bind to address: {}", e),
            })?;

        let running = self.running.clone();
        let server_handle = self.server_handle.clone();

        // 在后台运行服务器
        let handle = tokio::spawn(async move {
            *running.write().await = true;

            if let Err(e) = axum::serve(listener, app).await {
                error!("API server error: {}", e);
            }

            *running.write().await = false;
        });

        *server_handle.write().await = Some(handle);

        info!("API server started successfully on http://{}", addr);
        Ok(())
    }

    /// 停止 API 服务器
    pub async fn stop(&self) -> Result<(), ApiError> {
        info!("Stopping API server");

        // 检查是否在运行
        if !*self.running.read().await {
            warn!("API server is not running");
            return Ok(());
        }

        // 停止服务器
        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
        }

        *self.running.write().await = false;

        info!("API server stopped successfully");
        Ok(())
    }

    /// 检查服务器是否在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 获取服务器地址
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }

    /// 获取服务器URL
    pub fn get_url(&self) -> String {
        format!("http://{}:{}", self.config.bind_address, self.config.port)
    }

    /// 创建 CORS 层
    fn create_cors_layer(&self) -> CorsLayer {
        let mut cors = CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ]);

        // 配置允许的来源
        if self.config.allow_origins.contains(&"*".to_string()) {
            cors = cors.allow_origin(Any);
        } else {
            for origin in &self.config.allow_origins {
                if let Ok(origin_header) = origin.parse::<axum::http::HeaderValue>() {
                    cors = cors.allow_origin(origin_header);
                }
            }
        }

        cors
    }

    /// 验证认证令牌
    pub fn validate_auth_token(&self, token: Option<&str>) -> bool {
        match (&self.config.auth_token, token) {
            (Some(expected), Some(provided)) => expected == provided,
            (None, _) => true, // 如果没有配置认证令牌，则允许所有请求
            (Some(_), None) => false, // 配置了认证令牌但请求中没有提供
        }
    }

    /// 获取 API 信息
    pub async fn get_api_info(&self) -> ApiInfo {
        ApiInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: "CGMiner-RS API".to_string(),
            description: "High-performance ASIC Bitcoin miner API".to_string(),
            running: self.is_running().await,
            address: self.get_address(),
            url: self.get_url(),
            auth_required: self.config.auth_token.is_some(),
            cors_enabled: !self.config.allow_origins.is_empty(),
            allowed_origins: self.config.allow_origins.clone(),
        }
    }

    /// 获取 API 统计信息
    pub async fn get_api_stats(&self) -> ApiStats {
        // 这里应该收集实际的API统计信息
        // 为了简化，我们返回模拟数据
        ApiStats {
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0)),
            total_requests: 0, // 需要实际统计
            active_connections: 0, // 需要实际统计
            error_count: 0, // 需要实际统计
            average_response_time: std::time::Duration::from_millis(0), // 需要实际统计
        }
    }

    /// 重新加载配置
    pub async fn reload_config(&mut self, new_config: ApiConfig) -> Result<(), ApiError> {
        info!("Reloading API server configuration");

        let was_running = self.is_running().await;

        // 如果服务器在运行，先停止
        if was_running {
            self.stop().await?;
        }

        // 更新配置
        self.config = new_config;

        // 如果之前在运行，重新启动
        if was_running && self.config.enabled {
            self.start().await?;
        }

        info!("API server configuration reloaded successfully");
        Ok(())
    }

    /// 获取健康状态
    pub async fn get_health_status(&self) -> HealthStatus {
        let running = self.is_running().await;
        let mining_state = self.mining_manager.get_state().await;

        HealthStatus {
            api_server: running,
            mining_manager: !matches!(mining_state, crate::mining::MiningState::Error(_)),
            overall: running && !matches!(mining_state, crate::mining::MiningState::Error(_)),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// API 信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiInfo {
    pub version: String,
    pub name: String,
    pub description: String,
    pub running: bool,
    pub address: String,
    pub url: String,
    pub auth_required: bool,
    pub cors_enabled: bool,
    pub allowed_origins: Vec<String>,
}

/// API 统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiStats {
    pub uptime: std::time::Duration,
    pub total_requests: u64,
    pub active_connections: u32,
    pub error_count: u64,
    pub average_response_time: std::time::Duration,
}

/// 健康状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub api_server: bool,
    pub mining_manager: bool,
    pub overall: bool,
    pub timestamp: std::time::SystemTime,
}

/// API 中间件
pub mod middleware {
    use crate::api::ApiResponse;
    use crate::error::ApiError;
    use axum::{
        extract::Request,
        http::{HeaderMap, StatusCode},
        middleware::Next,
        response::Response,
        Json,
    };

    /// 认证中间件
    pub async fn auth_middleware(
        headers: HeaderMap,
        request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
        // 检查认证头
        if let Some(auth_header) = headers.get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    // 这里应该验证令牌
                    // 为了简化，我们假设所有令牌都有效
                    return Ok(next.run(request).await);
                }
            }
        }

        // 认证失败
        Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        ))
    }

    /// 速率限制中间件
    pub async fn rate_limit_middleware(
        request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
        // 这里应该实现实际的速率限制逻辑
        // 为了简化，我们直接通过
        Ok(next.run(request).await)
    }

    /// 日志中间件
    pub async fn logging_middleware(
        request: Request,
        next: Next,
    ) -> Response {
        let method = request.method().clone();
        let uri = request.uri().clone();
        let start = std::time::Instant::now();

        let response = next.run(request).await;

        let duration = start.elapsed();
        let status = response.status();

        tracing::info!(
            method = %method,
            uri = %uri,
            status = %status,
            duration = ?duration,
            "API request processed"
        );

        response
    }
}
