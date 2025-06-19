pub mod server;
pub mod handlers;
pub mod websocket;
pub mod auth;

use crate::mining::MiningManager;
use axum::{
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub use handlers::*;

/// API 响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// 系统状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub version: String,
    pub uptime: u64,
    pub mining_state: String,
    pub total_hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub active_devices: u32,
    pub connected_pools: u32,
    pub current_difficulty: f64,
    pub best_share: f64,
}

/// 设备状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusResponse {
    pub device_id: u32,
    pub name: String,
    pub status: String,
    pub temperature: Option<f32>,
    pub hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub uptime: u64,
    pub last_share_time: Option<u64>,
}

/// 矿池状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatusResponse {
    pub pool_id: u32,
    pub url: String,
    pub status: String,
    pub priority: u8,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub difficulty: f64,
    pub ping: Option<u64>,
    pub connected_at: Option<u64>,
}

/// 统计信息响应
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub mining_stats: MiningStatsData,
    pub device_stats: Vec<DeviceStatsData>,
    pub pool_stats: Vec<PoolStatsData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MiningStatsData {
    pub start_time: Option<u64>,
    pub uptime: u64,
    pub total_hashes: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub stale_shares: u64,
    pub best_share: f64,
    pub current_difficulty: f64,
    pub average_hashrate: f64,
    pub current_hashrate: f64,
    pub efficiency: f64,
    pub power_consumption: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceStatsData {
    pub device_id: u32,
    pub total_hashes: u64,
    pub valid_nonces: u64,
    pub invalid_nonces: u64,
    pub hardware_errors: u64,
    pub average_temperature: Option<f32>,
    pub average_hashrate: Option<f64>,
    pub uptime_seconds: u64,
    pub restart_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolStatsData {
    pub pool_id: u32,
    pub uptime: u64,
    pub connected_time: u64,
    pub total_shares: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub stale_shares: u64,
    pub best_share: f64,
    pub average_difficulty: f64,
    pub connection_attempts: u32,
    pub disconnection_count: u32,
}

/// 配置更新请求
#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub device_configs: Option<Vec<DeviceConfigUpdate>>,
    pub pool_configs: Option<Vec<PoolConfigUpdate>>,
    pub mining_config: Option<MiningConfigUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConfigUpdate {
    pub device_id: u32,
    pub enabled: Option<bool>,
    pub frequency: Option<u32>,
    pub voltage: Option<u32>,
    pub auto_tune: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PoolConfigUpdate {
    pub pool_id: u32,
    pub enabled: Option<bool>,
    pub priority: Option<u8>,
    pub url: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MiningConfigUpdate {
    pub work_restart_timeout: Option<u64>,
    pub scan_interval: Option<u64>,
    pub enable_auto_tuning: Option<bool>,
    pub target_temperature: Option<f32>,
    pub max_temperature: Option<f32>,
}

/// 控制命令请求
#[derive(Debug, Deserialize)]
pub struct ControlRequest {
    pub command: String,
    pub parameters: Option<serde_json::Value>,
}

/// 控制命令响应
#[derive(Debug, Serialize)]
pub struct ControlResponse {
    pub command: String,
    pub success: bool,
    pub message: String,
    pub result: Option<serde_json::Value>,
}

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// 订阅事件
    Subscribe { events: Vec<String> },
    /// 取消订阅
    Unsubscribe { events: Vec<String> },
    /// 系统状态更新
    StatusUpdate { data: SystemStatusResponse },
    /// 设备状态更新
    DeviceUpdate { data: DeviceStatusResponse },
    /// 矿池状态更新
    PoolUpdate { data: PoolStatusResponse },
    /// 挖矿事件
    MiningEvent { event: String, data: serde_json::Value },
    /// 错误消息
    Error { message: String },
    /// 心跳
    Ping,
    /// 心跳响应
    Pong,
}

/// API 应用状态
#[derive(Clone)]
pub struct AppState {
    pub mining_manager: Arc<MiningManager>,
}

/// 创建 API 路由
pub fn create_routes(state: AppState) -> Router {
    Router::new()
        // 系统状态路由
        .route("/api/v1/status", get(get_system_status))
        .route("/api/v1/stats", get(get_stats))

        // 设备管理路由
        .route("/api/v1/devices", get(get_devices))
        .route("/api/v1/devices/:id", get(get_device))
        .route("/api/v1/devices/:id/restart", post(restart_device))
        .route("/api/v1/devices/:id/config", post(update_device_config))

        // 矿池管理路由
        .route("/api/v1/pools", get(get_pools))
        .route("/api/v1/pools/:id", get(get_pool))
        .route("/api/v1/pools/:id/config", post(update_pool_config))

        // 控制路由
        .route("/api/v1/control", post(control_command))
        .route("/api/v1/config", post(update_config))

        // WebSocket 路由
        .route("/api/v1/ws", get(websocket_handler))

        // 健康检查
        .route("/health", get(health_check))

        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

/// 健康检查处理器
async fn health_check() -> Result<Json<ApiResponse<String>>, StatusCode> {
    Ok(Json(ApiResponse::success("OK".to_string())))
}

/// WebSocket 处理器
async fn websocket_handler() -> Result<Json<ApiResponse<String>>, StatusCode> {
    // 这里应该升级到 WebSocket 连接
    // 暂时返回错误，具体实现在 websocket.rs 中
    Err(StatusCode::NOT_IMPLEMENTED)
}
