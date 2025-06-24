//! Web服务器实现

use crate::web::{WebConfig, handlers};
use crate::monitoring::MonitoringSystem;
use crate::error::MiningError;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;
use tracing::{info, warn, error};

/// Web服务器
pub struct WebServer {
    /// 配置
    config: WebConfig,
    /// 监控系统
    monitoring: Arc<MonitoringSystem>,
    /// 服务器句柄
    server_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl WebServer {
    /// 创建新的Web服务器
    pub fn new(config: WebConfig, monitoring: Arc<MonitoringSystem>) -> Self {
        Self {
            config,
            monitoring,
            server_handle: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动Web服务器
    pub async fn start(&self) -> Result<(), MiningError> {
        if !self.config.enabled {
            info!("Web server is disabled");
            return Ok(());
        }

        // 检查是否已经在运行
        if *self.running.read().await {
            warn!("Web server is already running");
            return Ok(());
        }

        info!("Starting web server on {}:{}", self.config.bind_address, self.config.port);

        let monitoring = self.monitoring.clone();
        let config = self.config.clone();
        let running = self.running.clone();

        // 创建路由
        let routes = self.create_routes(monitoring.clone()).await;

        // 解析绑定地址
        let addr = format!("{}:{}", config.bind_address, config.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| MiningError::Config(crate::error::ConfigError::ValidationError {
            field: "bind_address".to_string(),
            reason: format!("Invalid bind address: {}", e),
        }))?;

        // 启动服务器
        let server = warp::serve(routes)
            .run(addr);

        let handle = tokio::spawn(async move {
            *running.write().await = true;
            server.await;
            *running.write().await = false;
        });

        *self.server_handle.write().await = Some(handle);
        *self.running.write().await = true;

        info!("Web server started on http://{}:{}", self.config.bind_address, self.config.port);
        Ok(())
    }

    /// 停止Web服务器
    pub async fn stop(&self) -> Result<(), MiningError> {
        info!("Stopping web server");

        if !*self.running.read().await {
            warn!("Web server is not running");
            return Ok(());
        }

        *self.running.write().await = false;

        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
        }

        info!("Web server stopped");
        Ok(())
    }

    /// 检查服务器是否在运行
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 获取服务器配置
    pub fn get_config(&self) -> &WebConfig {
        &self.config
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
        let api_routes = self.create_api_routes(monitoring.clone()).await;

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

        // 健康检查路由
        let health = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

        // CORS配置
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "authorization"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

        // 组合所有路由
        index
            .or(api_routes)
            .or(static_files)
            .or(health)
            .with(cors)
            .with(warp::log("cgminer_web"))
            .recover(handle_rejection)
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

        // 单个设备指标API
        let single_device_metrics = warp::path!("api" / "metrics" / "devices" / u32)
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(|device_id: u32, monitoring: Arc<MonitoringSystem>| async move {
                match monitoring.get_device_metrics(device_id).await {
                    Some(metrics) => Ok::<_, warp::Rejection>(warp::reply::json(&metrics)),
                    None => {
                        let response = serde_json::json!({
                            "error": format!("No metrics available for device {}", device_id)
                        });
                        Ok::<_, warp::Rejection>(warp::reply::json(&response))
                    }
                }
            });

        // 矿池指标API
        let pool_metrics = warp::path!("api" / "metrics" / "pools")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(handlers::api_pool_metrics);

        // 单个矿池指标API
        let single_pool_metrics = warp::path!("api" / "metrics" / "pools" / u32)
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(|pool_id: u32, monitoring: Arc<MonitoringSystem>| async move {
                match monitoring.get_pool_metrics(pool_id).await {
                    Some(metrics) => Ok::<_, warp::Rejection>(warp::reply::json(&metrics)),
                    None => {
                        let response = serde_json::json!({
                            "error": format!("No metrics available for pool {}", pool_id)
                        });
                        Ok::<_, warp::Rejection>(warp::reply::json(&response))
                    }
                }
            });

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

        // 指标历史统计API
        let metrics_history = warp::path!("api" / "stats" / "history")
            .and(warp::get())
            .and(monitoring_filter.clone())
            .and_then(|monitoring: Arc<MonitoringSystem>| async move {
                let stats = monitoring.get_metrics_history_stats().await;
                Ok::<_, warp::Rejection>(warp::reply::json(&stats))
            });

        // 组合API路由
        status
            .or(system_metrics)
            .or(mining_metrics)
            .or(device_metrics)
            .or(single_device_metrics)
            .or(pool_metrics)
            .or(single_pool_metrics)
            .or(performance_stats)
            .or(alerts)
            .or(metrics_history)
    }
}

/// 处理拒绝错误
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, warp::Rejection> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "Not Found";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "Invalid Body";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "Method Not Allowed";
    } else {
        error!("Unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = warp::reply::json(&serde_json::json!({
        "error": message,
        "code": code.as_u16()
    }));

    Ok(warp::reply::with_status(json, code))
}
