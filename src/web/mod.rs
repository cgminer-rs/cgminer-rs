//! Web界面模块

pub mod server;
pub mod handlers;
pub mod templates;

use crate::error::MiningError;
use crate::monitoring::MonitoringSystem;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;
use tracing::info;

/// Web服务器配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebConfig {
    /// 绑定地址
    pub bind_address: String,
    /// 端口
    pub port: u16,
    /// 是否启用
    pub enabled: bool,
    /// 静态文件路径
    #[serde(alias = "static_files_dir")]
    pub static_path: Option<String>,
    /// 模板目录路径
    pub template_dir: Option<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            enabled: true,
            static_path: Some("web/static".to_string()),
            template_dir: Some("web/templates".to_string()),
        }
    }
}

/// Web服务器
pub struct WebServer {
    /// 配置
    config: WebConfig,
    /// 监控系统
    monitoring: Arc<MonitoringSystem>,
    /// 服务器句柄
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl WebServer {
    /// 创建新的Web服务器
    pub fn new(config: WebConfig, monitoring: Arc<MonitoringSystem>) -> Self {
        Self {
            config,
            monitoring,
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// 启动Web服务器
    pub async fn start(&self) -> Result<(), MiningError> {
        if !self.config.enabled {
            info!("Web server is disabled");
            return Ok(());
        }

        info!("Starting web server on {}:{}", self.config.bind_address, self.config.port);

        let monitoring = self.monitoring.clone();

        // 创建路由
        let routes = self.create_routes(monitoring).await;

        // 启动服务器
        let addr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| MiningError::Config(crate::error::ConfigError::ValidationError {
            field: "bind_address".to_string(),
            reason: format!("Invalid bind address: {}", e),
        }))?;

        let server = warp::serve(routes).run(addr);
        let handle = tokio::spawn(server);

        *self.server_handle.write().await = Some(handle);

        info!("Web server started on http://{}:{}", self.config.bind_address, self.config.port);
        Ok(())
    }

    /// 停止Web服务器
    pub async fn stop(&self) -> Result<(), MiningError> {
        info!("Stopping web server");

        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
        }

        info!("Web server stopped");
        Ok(())
    }

    /// 创建路由
    async fn create_routes(
        &self,
        monitoring: Arc<MonitoringSystem>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        // 首页路由
        let index = warp::path::end()
            .and(warp::get())
            .and_then(handlers::index);

        // API路由
        let api_routes = self.create_api_routes(monitoring).await;

        // 静态文件路由
        let static_files = if let Some(ref static_path) = self.config.static_path {
            warp::path("static")
                .and(warp::fs::dir(static_path.clone()))
                .boxed()
        } else {
            warp::path("static")
                .and(warp::any())
                .and_then(|| async { Err(warp::reject::not_found()) })
                .boxed()
        };

        // 组合所有路由
        index
            .or(api_routes)
            .or(static_files)
            .with(warp::cors().allow_any_origin())
            .with(warp::log("cgminer_web"))
    }

    /// 创建API路由
    async fn create_api_routes(
        &self,
        monitoring: Arc<MonitoringSystem>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let monitoring_filter = warp::any().map(move || monitoring.clone());

        // 系统状态API
        let status = warp::path!("api" / "status")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_status);

        // 系统指标API
        let system_metrics = warp::path!("api" / "metrics" / "system")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_system_metrics);

        // 挖矿指标API
        let mining_metrics = warp::path!("api" / "metrics" / "mining")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_mining_metrics);

        // 设备指标API
        let device_metrics = warp::path!("api" / "metrics" / "devices")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_device_metrics);

        // 矿池指标API
        let pool_metrics = warp::path!("api" / "metrics" / "pools")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_pool_metrics);

        // 性能统计API
        let performance_stats = warp::path!("api" / "stats" / "performance")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_performance_stats);

        // 告警API
        let alerts = warp::path!("api" / "alerts")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_alerts);

        // 组合API路由
        status
            .or(system_metrics)
            .or(mining_metrics)
            .or(device_metrics)
            .or(pool_metrics)
            .or(performance_stats)
            .or(alerts)
    }
}
