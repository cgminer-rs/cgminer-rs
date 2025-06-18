use crate::config::MonitoringConfig;
use crate::error::MiningError;
use crate::monitoring::{
    SystemMetrics, MiningMetrics, DeviceMetrics, PoolMetrics, MetricsHistory,
    MonitoringState, MonitoringEvent, PerformanceStats
};
use crate::monitoring::metrics::MetricsCollector;
use crate::monitoring::alerts::AlertManager;
use crate::monitoring::simple_web::SimpleWebMonitor;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, broadcast};
use tokio::time::interval;
use tracing::{info, warn, debug};

/// 监控系统（简化版本）
pub struct MonitoringSystem {
    /// 配置
    config: MonitoringConfig,
    /// 运行状态
    state: Arc<RwLock<MonitoringState>>,
    /// 指标收集器
    metrics_collector: Arc<Mutex<MetricsCollector>>,
    /// 告警管理器
    alert_manager: Arc<Mutex<AlertManager>>,
    /// 简单Web监控器
    web_monitor: Option<Arc<Mutex<SimpleWebMonitor>>>,
    /// 指标历史记录
    metrics_history: Arc<RwLock<MetricsHistory>>,
    /// 性能统计
    performance_stats: Arc<RwLock<PerformanceStats>>,
    /// 事件广播
    event_sender: broadcast::Sender<MonitoringEvent>,
    /// 指标收集任务句柄
    collection_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 告警处理任务句柄
    alert_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 清理任务句柄
    cleanup_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行标志
    running: Arc<RwLock<bool>>,
}

impl MonitoringSystem {
    /// 创建新的监控系统
    pub async fn new(config: MonitoringConfig) -> Result<Self, MiningError> {
        info!("Creating monitoring system");

        let metrics_collector = MetricsCollector::new();
        let alert_manager = AlertManager::new(config.alert_thresholds.clone());
        let metrics_history = MetricsHistory::new(1000); // 保留最近1000条记录
        let (event_sender, _) = broadcast::channel(1000);

        // 创建简单Web监控器（如果启用）
        let web_monitor = if let Some(port) = config.web_port {
            Some(Arc::new(Mutex::new(SimpleWebMonitor::new(port))))
        } else {
            None
        };

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(MonitoringState::Stopped)),
            metrics_collector: Arc::new(Mutex::new(metrics_collector)),
            alert_manager: Arc::new(Mutex::new(alert_manager)),
            web_monitor,
            metrics_history: Arc::new(RwLock::new(metrics_history)),
            performance_stats: Arc::new(RwLock::new(PerformanceStats::default())),
            event_sender,
            collection_handle: Arc::new(Mutex::new(None)),
            alert_handle: Arc::new(Mutex::new(None)),
            cleanup_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动监控系统
    pub async fn start(&self) -> Result<(), MiningError> {
        if !self.config.enabled {
            info!("Monitoring system is disabled");
            return Ok(());
        }

        info!("Starting monitoring system");

        // 检查是否已经在运行
        if *self.running.read().await {
            warn!("Monitoring system is already running");
            return Ok(());
        }

        // 更新状态
        *self.state.write().await = MonitoringState::Starting;
        *self.running.write().await = true;

        // 启动指标收集任务
        self.start_metrics_collection().await?;

        // 启动告警处理任务
        self.start_alert_processing().await?;

        // 启动清理任务
        self.start_cleanup_task().await?;

        // 启动简单Web监控器
        if let Some(ref monitor) = self.web_monitor {
            monitor.lock().await.start().await?;
            info!("Simple web monitor started");
        }

        // 更新状态
        *self.state.write().await = MonitoringState::Running;

        // 发送启动事件
        self.send_event(MonitoringEvent::SystemMetricsUpdate {
            metrics: SystemMetrics::default(),
            timestamp: SystemTime::now(),
        }).await;

        info!("Monitoring system started successfully");
        Ok(())
    }

    /// 停止监控系统
    pub async fn stop(&self) -> Result<(), MiningError> {
        info!("Stopping monitoring system");

        // 检查是否在运行
        if !*self.running.read().await {
            warn!("Monitoring system is not running");
            return Ok(());
        }

        // 更新状态
        *self.state.write().await = MonitoringState::Stopping;
        *self.running.write().await = false;

        // 停止所有任务
        self.stop_tasks().await;

        // 停止简单Web监控器
        if let Some(ref monitor) = self.web_monitor {
            monitor.lock().await.stop().await?;
            info!("Simple web monitor stopped");
        }

        // 更新状态
        *self.state.write().await = MonitoringState::Stopped;

        info!("Monitoring system stopped successfully");
        Ok(())
    }

    /// 获取监控状态
    pub async fn get_state(&self) -> MonitoringState {
        self.state.read().await.clone()
    }

    /// 获取系统指标
    pub async fn get_system_metrics(&self) -> Option<SystemMetrics> {
        let history = self.metrics_history.read().await;
        history.get_latest_system_metrics().cloned()
    }

    /// 获取挖矿指标
    pub async fn get_mining_metrics(&self) -> Option<MiningMetrics> {
        let history = self.metrics_history.read().await;
        history.get_latest_mining_metrics().cloned()
    }

    /// 获取设备指标
    pub async fn get_device_metrics(&self, device_id: u32) -> Option<DeviceMetrics> {
        let history = self.metrics_history.read().await;
        history.get_latest_device_metrics(device_id).cloned()
    }

    /// 获取矿池指标
    pub async fn get_pool_metrics(&self, pool_id: u32) -> Option<PoolMetrics> {
        let history = self.metrics_history.read().await;
        history.get_latest_pool_metrics(pool_id).cloned()
    }

    /// 获取性能统计
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        self.performance_stats.read().await.clone()
    }

    /// 订阅监控事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<MonitoringEvent> {
        self.event_sender.subscribe()
    }

    /// 发送事件
    async fn send_event(&self, event: MonitoringEvent) {
        if let Err(e) = self.event_sender.send(event) {
            debug!("Failed to send monitoring event: {}", e);
        }
    }

    /// 启动指标收集任务
    async fn start_metrics_collection(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let metrics_collector = self.metrics_collector.clone();
        let metrics_history = self.metrics_history.clone();
        let performance_stats = self.performance_stats.clone();
        let event_sender = self.event_sender.clone();
        let web_monitor = self.web_monitor.clone();
        let collection_interval = Duration::from_secs(self.config.metrics_interval);

        let handle = tokio::spawn(async move {
            let mut interval = interval(collection_interval);

            while *running.read().await {
                interval.tick().await;

                let start_time = std::time::Instant::now();

                // 收集系统指标
                {
                    let mut collector = metrics_collector.lock().await;

                    if let Ok(system_metrics) = collector.collect_system_metrics().await {
                        // 添加到历史记录
                        {
                            let mut history = metrics_history.write().await;
                            history.add_system_metrics(system_metrics.clone());
                        }

                        // 更新Web监控器
                        if let Some(ref monitor) = web_monitor {
                            monitor.lock().await.update_system_metrics(system_metrics.clone()).await;
                        }

                        // 发送事件
                        let _ = event_sender.send(MonitoringEvent::SystemMetricsUpdate {
                            metrics: system_metrics,
                            timestamp: SystemTime::now(),
                        });
                    }

                    if let Ok(mining_metrics) = collector.collect_mining_metrics().await {
                        // 添加到历史记录
                        {
                            let mut history = metrics_history.write().await;
                            history.add_mining_metrics(mining_metrics.clone());
                        }

                        // 更新Web监控器
                        if let Some(ref monitor) = web_monitor {
                            monitor.lock().await.update_mining_metrics(mining_metrics.clone()).await;
                        }

                        // 发送事件
                        let _ = event_sender.send(MonitoringEvent::MiningMetricsUpdate {
                            metrics: mining_metrics,
                            timestamp: SystemTime::now(),
                        });
                    }

                    // 收集设备指标
                    for device_id in 0..2u32 { // 假设有2个设备
                        if let Ok(device_metrics) = collector.collect_device_metrics(device_id).await {
                            // 添加到历史记录
                            {
                                let mut history = metrics_history.write().await;
                                history.add_device_metrics(device_id, device_metrics.clone());
                            }

                            // 更新Web监控器
                            if let Some(ref monitor) = web_monitor {
                                monitor.lock().await.update_device_metrics(device_id, device_metrics.clone()).await;
                            }

                            // 发送事件
                            let _ = event_sender.send(MonitoringEvent::DeviceMetricsUpdate {
                                device_id,
                                metrics: device_metrics,
                                timestamp: SystemTime::now(),
                            });
                        }
                    }

                    // 收集矿池指标
                    for pool_id in 0..2u32 { // 假设有2个矿池
                        if let Ok(pool_metrics) = collector.collect_pool_metrics(pool_id).await {
                            // 添加到历史记录
                            {
                                let mut history = metrics_history.write().await;
                                history.add_pool_metrics(pool_id, pool_metrics.clone());
                            }

                            // 更新Web监控器
                            if let Some(ref monitor) = web_monitor {
                                monitor.lock().await.update_pool_metrics(pool_id, pool_metrics.clone()).await;
                            }

                            // 发送事件
                            let _ = event_sender.send(MonitoringEvent::PoolMetricsUpdate {
                                pool_id,
                                metrics: pool_metrics,
                                timestamp: SystemTime::now(),
                            });
                        }
                    }
                }

                // 更新性能统计
                let collection_time = start_time.elapsed();
                {
                    let mut stats = performance_stats.write().await;
                    stats.record_collection_time(collection_time);
                }
            }
        });

        *self.collection_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动告警处理任务
    async fn start_alert_processing(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let alert_manager = self.alert_manager.clone();
        let metrics_history = self.metrics_history.clone();
        let performance_stats = self.performance_stats.clone();
        let event_sender = self.event_sender.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // 每10秒检查一次告警

            while *running.read().await {
                interval.tick().await;

                let start_time = std::time::Instant::now();

                // 检查告警
                {
                    let mut manager = alert_manager.lock().await;
                    let history = metrics_history.read().await;

                    // 检查系统告警
                    if let Some(system_metrics) = history.get_latest_system_metrics() {
                        if let Ok(alerts) = manager.check_system_alerts(system_metrics).await {
                            for alert in alerts {
                                let _ = event_sender.send(MonitoringEvent::AlertTriggered {
                                    alert,
                                    timestamp: SystemTime::now(),
                                });
                            }
                        }
                    }

                    // 检查设备告警
                    for device_id in 0..2u32 {
                        if let Some(device_metrics) = history.get_latest_device_metrics(device_id) {
                            if let Ok(alerts) = manager.check_device_alerts(device_metrics).await {
                                for alert in alerts {
                                    let _ = event_sender.send(MonitoringEvent::AlertTriggered {
                                        alert,
                                        timestamp: SystemTime::now(),
                                    });
                                }
                            }
                        }
                    }
                }

                // 更新性能统计
                let processing_time = start_time.elapsed();
                {
                    let mut stats = performance_stats.write().await;
                    stats.record_alert_processing_time(processing_time);
                }
            }
        });

        *self.alert_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动清理任务
    async fn start_cleanup_task(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let _metrics_history = self.metrics_history.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // 每小时清理一次

            while *running.read().await {
                interval.tick().await;

                // 清理过期的指标数据
                // 这里可以添加清理逻辑，比如删除超过一定时间的历史数据
                debug!("Performing metrics cleanup");
            }
        });

        *self.cleanup_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止所有任务
    async fn stop_tasks(&self) {
        // 停止指标收集任务
        if let Some(handle) = self.collection_handle.lock().await.take() {
            handle.abort();
        }

        // 停止告警处理任务
        if let Some(handle) = self.alert_handle.lock().await.take() {
            handle.abort();
        }

        // 停止清理任务
        if let Some(handle) = self.cleanup_handle.lock().await.take() {
            handle.abort();
        }
    }

    /// 重置指标历史
    pub async fn reset_metrics_history(&self) {
        let mut history = self.metrics_history.write().await;
        history.clear();
        info!("Metrics history reset");
    }

    /// 获取指标历史统计
    pub async fn get_metrics_history_stats(&self) -> MetricsHistoryStats {
        let history = self.metrics_history.read().await;

        MetricsHistoryStats {
            system_metrics_count: history.system_metrics.len(),
            mining_metrics_count: history.mining_metrics.len(),
            device_metrics_count: history.device_metrics.values().map(|v| v.len()).sum(),
            pool_metrics_count: history.pool_metrics.values().map(|v| v.len()).sum(),
            total_entries: history.system_metrics.len()
                + history.mining_metrics.len()
                + history.device_metrics.values().map(|v| v.len()).sum::<usize>()
                + history.pool_metrics.values().map(|v| v.len()).sum::<usize>(),
        }
    }

    /// 获取状态摘要（用于命令行显示）
    pub async fn get_status_summary(&self) -> String {
        if let Some(ref monitor) = self.web_monitor {
            monitor.lock().await.get_status_summary().await
        } else {
            "📊 Web监控未启用".to_string()
        }
    }
}

/// 指标历史统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsHistoryStats {
    pub system_metrics_count: usize,
    pub mining_metrics_count: usize,
    pub device_metrics_count: usize,
    pub pool_metrics_count: usize,
    pub total_entries: usize,
}
