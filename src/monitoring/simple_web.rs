//! 简单的Web监控界面
//!
//! 轻量级的内置监控系统，替代复杂的Prometheus
//! 提供简单易用的Web界面显示挖矿状态

use crate::monitoring::{SystemMetrics, MiningMetrics, DeviceMetrics, PoolMetrics};
use crate::error::MiningError;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use warp::{Filter, Reply};
use tracing::{info, error};

/// 简化的指标历史记录
#[derive(Debug, Clone)]
pub struct SimpleMetricsHistory {
    /// 系统指标历史
    pub system_metrics: Vec<SystemMetrics>,
    /// 挖矿指标历史
    pub mining_metrics: Vec<MiningMetrics>,
    /// 设备指标历史
    pub device_metrics: HashMap<u32, Vec<DeviceMetrics>>,
    /// 矿池指标历史
    pub pool_metrics: HashMap<u32, Vec<PoolMetrics>>,
    /// 最大记录数
    max_records: usize,
}

impl SimpleMetricsHistory {
    pub fn new(max_records: usize) -> Self {
        Self {
            system_metrics: Vec::new(),
            mining_metrics: Vec::new(),
            device_metrics: HashMap::new(),
            pool_metrics: HashMap::new(),
            max_records,
        }
    }

    pub fn add_system_metrics(&mut self, metrics: SystemMetrics) {
        self.system_metrics.push(metrics);
        if self.system_metrics.len() > self.max_records {
            self.system_metrics.remove(0);
        }
    }

    pub fn add_mining_metrics(&mut self, metrics: MiningMetrics) {
        self.mining_metrics.push(metrics);
        if self.mining_metrics.len() > self.max_records {
            self.mining_metrics.remove(0);
        }
    }

    pub fn add_device_metrics(&mut self, device_id: u32, metrics: DeviceMetrics) {
        let device_history = self.device_metrics.entry(device_id).or_insert_with(Vec::new);
        device_history.push(metrics);
        if device_history.len() > self.max_records {
            device_history.remove(0);
        }
    }

    pub fn add_pool_metrics(&mut self, pool_id: u32, metrics: PoolMetrics) {
        let pool_history = self.pool_metrics.entry(pool_id).or_insert_with(Vec::new);
        pool_history.push(metrics);
        if pool_history.len() > self.max_records {
            pool_history.remove(0);
        }
    }

    pub fn get_latest_system_metrics(&self) -> Option<&SystemMetrics> {
        self.system_metrics.last()
    }

    pub fn get_latest_mining_metrics(&self) -> Option<&MiningMetrics> {
        self.mining_metrics.last()
    }
}

/// 简单Web监控器
pub struct SimpleWebMonitor {
    /// 绑定端口
    port: u16,
    /// 指标历史
    metrics_history: Arc<RwLock<SimpleMetricsHistory>>,
    /// 服务器句柄
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// 是否启用
    enabled: bool,
}

/// 监控仪表板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// 当前时间戳
    pub timestamp: u64,
    /// 系统状态
    pub system: Option<SystemStatus>,
    /// 挖矿状态
    pub mining: Option<MiningStatus>,
    /// 设备状态列表
    pub devices: Vec<DeviceStatus>,
    /// 矿池状态列表
    pub pools: Vec<PoolStatus>,
    /// 简单统计
    pub stats: SimpleStats,
}

/// 系统状态（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub temperature: f32,
    pub uptime_hours: f64,
    pub power_consumption: f64,
}

/// 挖矿状态（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStatus {
    pub total_hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub reject_rate: f64,
    pub active_devices: u32,
    pub efficiency: f64,
}

/// 设备状态（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_id: u32,
    pub temperature: f32,
    pub hashrate: f64,
    pub power: f64,
    pub status: String,
    pub error_rate: f64,
}

/// 矿池状态（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub pool_id: u32,
    pub connected: bool,
    pub ping_ms: Option<u64>,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub uptime_hours: f64,
}

/// 简单统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleStats {
    pub total_runtime_hours: f64,
    pub total_shares: u64,
    pub average_hashrate: f64,
    pub best_share: f64,
    pub hardware_errors: u64,
}

impl SimpleWebMonitor {
    /// 创建新的简单Web监控器
    pub fn new(port: u16) -> Self {
        Self {
            port,
            metrics_history: Arc::new(RwLock::new(SimpleMetricsHistory::new(100))), // 保留最近100条记录
            server_handle: Arc::new(RwLock::new(None)),
            enabled: true,
        }
    }

    /// 启动Web监控服务器
    pub async fn start(&self) -> Result<(), MiningError> {
        if !self.enabled {
            info!("📊 Web监控已禁用");
            return Ok(());
        }

        info!("🌐 启动简单Web监控界面，端口: {}", self.port);

        let metrics_history = self.metrics_history.clone();

        // 主页路由 - 返回HTML页面
        let index_route = warp::path::end()
            .and(warp::get())
            .map(|| warp::reply::html(get_dashboard_html()));

        // API路由 - 返回JSON数据
        let api_route = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::get())
            .and_then(move || {
                let metrics_history = metrics_history.clone();
                async move {
                    match generate_dashboard_data(metrics_history).await {
                        Ok(data) => Ok(warp::reply::json(&data)),
                        Err(e) => {
                            error!("生成仪表板数据失败: {}", e);
                            Err(warp::reject::custom(ApiError))
                        }
                    }
                }
            });

        // 静态资源路由（CSS/JS）
        let static_route = warp::path("static")
            .and(warp::path::param())
            .and(warp::get())
            .map(|file: String| {
                match file.as_str() {
                    "style.css" => warp::reply::with_header(
                        get_dashboard_css(),
                        "content-type",
                        "text/css",
                    ).into_response(),
                    "script.js" => warp::reply::with_header(
                        get_dashboard_js(),
                        "content-type",
                        "application/javascript",
                    ).into_response(),
                    _ => warp::reply::with_status(
                        "Not Found",
                        warp::http::StatusCode::NOT_FOUND,
                    ).into_response(),
                }
            });

        let routes = index_route.or(api_route).or(static_route);

        // 启动服务器
        let server = warp::serve(routes)
            .run(([0, 0, 0, 0], self.port));

        let handle = tokio::spawn(server);
        *self.server_handle.write().await = Some(handle);

        info!("✅ Web监控界面已启动: http://localhost:{}", self.port);
        Ok(())
    }

    /// 停止Web监控服务器
    pub async fn stop(&self) -> Result<(), MiningError> {
        info!("🛑 停止Web监控界面");

        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
        }

        info!("✅ Web监控界面已停止");
        Ok(())
    }

    /// 更新系统指标
    pub async fn update_system_metrics(&self, metrics: SystemMetrics) {
        self.metrics_history.write().await.add_system_metrics(metrics);
    }

    /// 更新挖矿指标
    pub async fn update_mining_metrics(&self, metrics: MiningMetrics) {
        self.metrics_history.write().await.add_mining_metrics(metrics);
    }

    /// 更新设备指标
    pub async fn update_device_metrics(&self, device_id: u32, metrics: DeviceMetrics) {
        self.metrics_history.write().await.add_device_metrics(device_id, metrics);
    }

    /// 更新矿池指标
    pub async fn update_pool_metrics(&self, pool_id: u32, metrics: PoolMetrics) {
        self.metrics_history.write().await.add_pool_metrics(pool_id, metrics);
    }

    /// 启用/禁用监控
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            info!("📊 Web监控已启用");
        } else {
            info!("📊 Web监控已禁用");
        }
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取当前状态摘要（用于命令行显示）
    pub async fn get_status_summary(&self) -> String {
        let history = self.metrics_history.read().await;

        let mut summary = String::new();
        summary.push_str("📊 挖矿状态摘要\n");
        summary.push_str("==================\n");

        // 挖矿状态
        if let Some(mining) = history.get_latest_mining_metrics() {
            summary.push_str(&format!("⚡ 总算力: {:.2} GH/s\n", mining.total_hashrate));
            summary.push_str(&format!("✅ 接受份额: {}\n", mining.accepted_shares));
            summary.push_str(&format!("❌ 拒绝份额: {}\n", mining.rejected_shares));
            let reject_rate = if mining.accepted_shares + mining.rejected_shares > 0 {
                (mining.rejected_shares as f64 / (mining.accepted_shares + mining.rejected_shares) as f64) * 100.0
            } else {
                0.0
            };
            summary.push_str(&format!("📊 拒绝率: {:.2}%\n", reject_rate));
            summary.push_str(&format!("🔧 活跃设备: {}\n", mining.active_devices));
        }

        // 系统状态
        if let Some(system) = history.get_latest_system_metrics() {
            summary.push_str(&format!("🌡️  温度: {:.1}°C\n", system.temperature));
            summary.push_str(&format!("💾 内存使用: {:.1}%\n", system.memory_usage));
            summary.push_str(&format!("⚡ 功耗: {:.1}W\n", system.power_consumption));
        }

        summary.push_str("==================\n");
        summary.push_str(&format!("🌐 Web界面: http://localhost:{}\n", self.port));

        summary
    }
}

/// 生成仪表板数据
async fn generate_dashboard_data(
    metrics_history: Arc<RwLock<SimpleMetricsHistory>>,
) -> Result<DashboardData, MiningError> {
    let history = metrics_history.read().await;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    // 系统状态
    let system = history.get_latest_system_metrics().map(|m| SystemStatus {
        cpu_usage: m.cpu_usage,
        memory_usage: m.memory_usage,
        temperature: m.temperature,
        uptime_hours: m.uptime.as_secs() as f64 / 3600.0,
        power_consumption: m.power_consumption,
    });

    // 挖矿状态
    let mining = history.get_latest_mining_metrics().map(|m| {
        let reject_rate = if m.accepted_shares + m.rejected_shares > 0 {
            (m.rejected_shares as f64 / (m.accepted_shares + m.rejected_shares) as f64) * 100.0
        } else {
            0.0
        };

        MiningStatus {
            total_hashrate: m.total_hashrate,
            accepted_shares: m.accepted_shares,
            rejected_shares: m.rejected_shares,
            reject_rate,
            active_devices: m.active_devices,
            efficiency: m.efficiency,
        }
    });

    // 设备状态
    let mut devices = Vec::new();
    for (device_id, device_history) in &history.device_metrics {
        if let Some(latest) = device_history.last() {
            let status = if latest.temperature > 80.0 {
                "过热".to_string()
            } else if latest.hashrate == 0.0 {
                "离线".to_string()
            } else {
                "正常".to_string()
            };

            devices.push(DeviceStatus {
                device_id: *device_id,
                temperature: latest.temperature,
                hashrate: latest.hashrate,
                power: latest.power_consumption,
                status,
                error_rate: latest.error_rate,
            });
        }
    }

    // 矿池状态
    let mut pools = Vec::new();
    for (pool_id, pool_history) in &history.pool_metrics {
        if let Some(latest) = pool_history.last() {
            pools.push(PoolStatus {
                pool_id: *pool_id,
                connected: latest.connected,
                ping_ms: latest.ping.map(|p| p.as_millis() as u64),
                accepted_shares: latest.accepted_shares,
                rejected_shares: latest.rejected_shares,
                uptime_hours: latest.connection_uptime.as_secs() as f64 / 3600.0,
            });
        }
    }

    // 简单统计
    let stats = SimpleStats {
        total_runtime_hours: system.as_ref().map(|s| s.uptime_hours).unwrap_or(0.0),
        total_shares: mining.as_ref().map(|m| m.accepted_shares + m.rejected_shares).unwrap_or(0),
        average_hashrate: mining.as_ref().map(|m| m.total_hashrate).unwrap_or(0.0),
        best_share: history.get_latest_mining_metrics().map(|m| m.best_share).unwrap_or(0.0),
        hardware_errors: history.get_latest_mining_metrics().map(|m| m.hardware_errors).unwrap_or(0),
    };

    Ok(DashboardData {
        timestamp: now,
        system,
        mining,
        devices,
        pools,
        stats,
    })
}

/// 自定义错误类型
#[derive(Debug)]
struct ApiError;

impl warp::reject::Reject for ApiError {}

/// 获取仪表板HTML
fn get_dashboard_html() -> &'static str {
    include_str!("../../web/dashboard.html")
}

/// 获取仪表板CSS
fn get_dashboard_css() -> &'static str {
    include_str!("../../web/style.css")
}

/// 获取仪表板JavaScript
fn get_dashboard_js() -> &'static str {
    include_str!("../../web/script.js")
}
