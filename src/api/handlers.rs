use crate::api::{
    AppState, ApiResponse, SystemStatusResponse, DeviceStatusResponse, 
    PoolStatusResponse, StatsResponse, ConfigUpdateRequest, ControlRequest, ControlResponse
};
use crate::error::ApiError;
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{info, warn, error};

/// 获取系统状态
pub async fn get_system_status(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SystemStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.mining_manager.get_system_status().await {
        Ok(status) => {
            let response = SystemStatusResponse {
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime: status.uptime.as_secs(),
                mining_state: format!("{:?}", status.state),
                total_hashrate: status.total_hashrate,
                accepted_shares: status.accepted_shares,
                rejected_shares: status.rejected_shares,
                hardware_errors: status.hardware_errors,
                active_devices: status.active_devices,
                connected_pools: status.connected_pools,
                current_difficulty: status.current_difficulty,
                best_share: status.best_share,
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to get system status: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to get system status: {}", e))),
            ))
        }
    }
}

/// 获取统计信息
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 获取挖矿统计
    let mining_stats = state.mining_manager.get_stats().await;
    
    // 转换为响应格式
    let mining_stats_data = crate::api::MiningStatsData {
        start_time: mining_stats.start_time.map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_secs()
        }),
        uptime: mining_stats.uptime.as_secs(),
        total_hashes: mining_stats.total_hashes,
        accepted_shares: mining_stats.accepted_shares,
        rejected_shares: mining_stats.rejected_shares,
        hardware_errors: mining_stats.hardware_errors,
        stale_shares: mining_stats.stale_shares,
        best_share: mining_stats.best_share,
        current_difficulty: mining_stats.current_difficulty,
        average_hashrate: mining_stats.average_hashrate,
        current_hashrate: mining_stats.current_hashrate,
        efficiency: mining_stats.efficiency,
        power_consumption: mining_stats.power_consumption,
    };
    
    // 这里应该获取实际的设备和矿池统计
    // 为了简化，我们返回空的列表
    let device_stats = Vec::new();
    let pool_stats = Vec::new();
    
    let response = StatsResponse {
        mining_stats: mining_stats_data,
        device_stats,
        pool_stats,
    };
    
    Ok(Json(ApiResponse::success(response)))
}

/// 获取所有设备
pub async fn get_devices(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceStatusResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 这里应该从设备管理器获取实际的设备列表
    // 为了简化，我们返回模拟数据
    let devices = vec![
        DeviceStatusResponse {
            device_id: 0,
            name: "Maijie L7 Chain 0".to_string(),
            status: "Mining".to_string(),
            temperature: Some(65.5),
            hashrate: 38.0,
            accepted_shares: 1250,
            rejected_shares: 15,
            hardware_errors: 2,
            uptime: 3600,
            last_share_time: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
        },
        DeviceStatusResponse {
            device_id: 1,
            name: "Maijie L7 Chain 1".to_string(),
            status: "Mining".to_string(),
            temperature: Some(67.2),
            hashrate: 37.5,
            accepted_shares: 1180,
            rejected_shares: 12,
            hardware_errors: 1,
            uptime: 3600,
            last_share_time: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
        },
    ];
    
    Ok(Json(ApiResponse::success(devices)))
}

/// 获取单个设备
pub async fn get_device(
    Path(device_id): Path<u32>,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<DeviceStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 这里应该从设备管理器获取实际的设备信息
    // 为了简化，我们返回模拟数据
    if device_id > 1 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Device {} not found", device_id))),
        ));
    }
    
    let device = DeviceStatusResponse {
        device_id,
        name: format!("Maijie L7 Chain {}", device_id),
        status: "Mining".to_string(),
        temperature: Some(65.5 + device_id as f32),
        hashrate: 38.0 - device_id as f64 * 0.5,
        accepted_shares: 1250 - device_id * 70,
        rejected_shares: 15 - device_id * 3,
        hardware_errors: 2 - device_id,
        uptime: 3600,
        last_share_time: Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()),
    };
    
    Ok(Json(ApiResponse::success(device)))
}

/// 重启设备
pub async fn restart_device(
    Path(device_id): Path<u32>,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Restarting device {}", device_id);
    
    // 这里应该调用设备管理器的重启方法
    // 为了简化，我们只是返回成功消息
    if device_id > 1 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Device {} not found", device_id))),
        ));
    }
    
    Ok(Json(ApiResponse::success(format!("Device {} restart initiated", device_id))))
}

/// 更新设备配置
pub async fn update_device_config(
    Path(device_id): Path<u32>,
    State(_state): State<AppState>,
    Json(config): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Updating device {} configuration: {:?}", device_id, config);
    
    // 这里应该验证配置并应用到设备
    // 为了简化，我们只是返回成功消息
    if device_id > 1 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Device {} not found", device_id))),
        ));
    }
    
    Ok(Json(ApiResponse::success(format!("Device {} configuration updated", device_id))))
}

/// 获取所有矿池
pub async fn get_pools(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PoolStatusResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 这里应该从矿池管理器获取实际的矿池列表
    // 为了简化，我们返回模拟数据
    let pools = vec![
        PoolStatusResponse {
            pool_id: 0,
            url: "stratum+tcp://pool.example.com:4444".to_string(),
            status: "Connected".to_string(),
            priority: 1,
            accepted_shares: 2430,
            rejected_shares: 27,
            stale_shares: 5,
            difficulty: 1024.0,
            ping: Some(45),
            connected_at: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() - 3600),
        },
        PoolStatusResponse {
            pool_id: 1,
            url: "stratum+tcp://backup.example.com:4444".to_string(),
            status: "Disconnected".to_string(),
            priority: 2,
            accepted_shares: 0,
            rejected_shares: 0,
            stale_shares: 0,
            difficulty: 0.0,
            ping: None,
            connected_at: None,
        },
    ];
    
    Ok(Json(ApiResponse::success(pools)))
}

/// 获取单个矿池
pub async fn get_pool(
    Path(pool_id): Path<u32>,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<PoolStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 这里应该从矿池管理器获取实际的矿池信息
    // 为了简化，我们返回模拟数据
    if pool_id > 1 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Pool {} not found", pool_id))),
        ));
    }
    
    let pool = PoolStatusResponse {
        pool_id,
        url: format!("stratum+tcp://pool{}.example.com:4444", pool_id),
        status: if pool_id == 0 { "Connected" } else { "Disconnected" }.to_string(),
        priority: pool_id as u8 + 1,
        accepted_shares: if pool_id == 0 { 2430 } else { 0 },
        rejected_shares: if pool_id == 0 { 27 } else { 0 },
        stale_shares: if pool_id == 0 { 5 } else { 0 },
        difficulty: if pool_id == 0 { 1024.0 } else { 0.0 },
        ping: if pool_id == 0 { Some(45) } else { None },
        connected_at: if pool_id == 0 {
            Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() - 3600)
        } else {
            None
        },
    };
    
    Ok(Json(ApiResponse::success(pool)))
}

/// 更新矿池配置
pub async fn update_pool_config(
    Path(pool_id): Path<u32>,
    State(_state): State<AppState>,
    Json(config): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Updating pool {} configuration: {:?}", pool_id, config);
    
    // 这里应该验证配置并应用到矿池
    // 为了简化，我们只是返回成功消息
    if pool_id > 1 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(format!("Pool {} not found", pool_id))),
        ));
    }
    
    Ok(Json(ApiResponse::success(format!("Pool {} configuration updated", pool_id))))
}

/// 控制命令
pub async fn control_command(
    State(_state): State<AppState>,
    Json(request): Json<ControlRequest>,
) -> Result<Json<ApiResponse<ControlResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Executing control command: {}", request.command);
    
    let response = match request.command.as_str() {
        "start" => ControlResponse {
            command: request.command.clone(),
            success: true,
            message: "Mining started successfully".to_string(),
            result: None,
        },
        "stop" => ControlResponse {
            command: request.command.clone(),
            success: true,
            message: "Mining stopped successfully".to_string(),
            result: None,
        },
        "restart" => ControlResponse {
            command: request.command.clone(),
            success: true,
            message: "Mining restarted successfully".to_string(),
            result: None,
        },
        "pause" => ControlResponse {
            command: request.command.clone(),
            success: true,
            message: "Mining paused successfully".to_string(),
            result: None,
        },
        "resume" => ControlResponse {
            command: request.command.clone(),
            success: true,
            message: "Mining resumed successfully".to_string(),
            result: None,
        },
        _ => ControlResponse {
            command: request.command.clone(),
            success: false,
            message: format!("Unknown command: {}", request.command),
            result: None,
        },
    };
    
    Ok(Json(ApiResponse::success(response)))
}

/// 更新配置
pub async fn update_config(
    State(_state): State<AppState>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Updating configuration: {:?}", request);
    
    // 这里应该验证配置并应用更改
    // 为了简化，我们只是返回成功消息
    
    Ok(Json(ApiResponse::success("Configuration updated successfully".to_string())))
}

/// 查询参数
#[derive(Deserialize)]
pub struct QueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

/// 获取设备列表（带查询参数）
pub async fn get_devices_with_query(
    Query(params): Query<QueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceStatusResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Getting devices with query params: {:?}", params);
    
    // 这里应该根据查询参数过滤和排序设备
    // 为了简化，我们忽略查询参数并返回所有设备
    get_devices(State(state)).await
}
