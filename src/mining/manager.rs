use crate::config::Config;
use crate::error::MiningError;
use crate::device::{DeviceManager, DeviceCoreMapper};
use crate::pool::PoolManager;
use crate::monitoring::{MonitoringSystem, MiningMetrics};
use crate::mining::{MiningState, MiningStats, MiningConfig, MiningEvent, WorkItem, ResultItem, Hashmeter};
use crate::logging::formatter::format_duration;
use cgminer_core::{CoreRegistry, CoreType, CoreConfig};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc, broadcast};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

/// 挖矿管理器 - 协调所有子系统（集成协调器功能）
pub struct MiningManager {
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// 设备管理器
    device_manager: Arc<Mutex<DeviceManager>>,
    /// 设备-核心映射器（从协调器移入）
    device_core_mapper: Arc<DeviceCoreMapper>,
    /// 矿池管理器
    pool_manager: Arc<Mutex<PoolManager>>,
    /// 监控系统
    monitoring_system: Arc<Mutex<MonitoringSystem>>,
    /// 算力计量器
    hashmeter: Arc<Mutex<Option<Hashmeter>>>,
    /// 完整配置
    full_config: Config,
    /// 挖矿配置
    config: MiningConfig,
    /// 挖矿状态
    state: Arc<RwLock<MiningState>>,
    /// 挖矿统计
    stats: Arc<RwLock<MiningStats>>,

    /// 工作分发通道
    work_sender: Arc<Mutex<Option<mpsc::UnboundedSender<WorkItem>>>>,
    work_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<WorkItem>>>>,
    /// 结果收集通道
    result_sender: Arc<Mutex<Option<mpsc::UnboundedSender<ResultItem>>>>,
    result_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<ResultItem>>>>,
    /// 事件广播
    event_sender: broadcast::Sender<MiningEvent>,
    /// 主循环任务句柄
    main_loop_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 工作分发任务句柄
    work_dispatch_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 结果处理任务句柄
    result_process_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 算力更新任务句柄
    hashmeter_update_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 核心结果收集任务句柄
    core_result_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl MiningManager {
    /// 创建新的挖矿管理器
    pub async fn new(config: Config, core_registry: Arc<CoreRegistry>) -> Result<Self, MiningError> {
        // 简化初始化日志
        debug!("Initializing mining manager");

        // 创建设备管理器
        let mut device_manager = DeviceManager::new(config.devices.clone(), core_registry.clone());
        device_manager.set_full_config(config.clone());

        // 创建设备-核心映射器
        let device_core_mapper = DeviceCoreMapper::new(core_registry.clone());

        // 根据配置的核心类型注册相应的设备驱动
        Self::register_drivers_for_cores(&mut device_manager, &config.cores).await?;

        // 创建矿池管理器
        let pool_manager = PoolManager::new(config.pools.clone()).await?;

        // 创建监控系统
        let monitoring_system = MonitoringSystem::new(config.monitoring.clone()).await?;

        // 创建通道
        let (work_sender, work_receiver) = mpsc::unbounded_channel();
        let (result_sender, result_receiver) = mpsc::unbounded_channel();
        let (event_sender, _) = broadcast::channel(1000);

        let mining_config = MiningConfig::from(&config);

        // 创建算力计量器
        let hashmeter = if config.hashmeter.enabled && config.hashmeter.log_interval > 0 {
            Some(Hashmeter::new(config.hashmeter.clone()))
        } else {
            None
        };

        Ok(Self {
            core_registry,
            device_manager: Arc::new(Mutex::new(device_manager)),
            device_core_mapper: Arc::new(device_core_mapper),
            pool_manager: Arc::new(Mutex::new(pool_manager)),
            monitoring_system: Arc::new(Mutex::new(monitoring_system)),
            hashmeter: Arc::new(Mutex::new(hashmeter)),
            full_config: config,
            config: mining_config,
            state: Arc::new(RwLock::new(MiningState::Stopped)),
            stats: Arc::new(RwLock::new(MiningStats::new())),

            work_sender: Arc::new(Mutex::new(Some(work_sender))),
            work_receiver: Arc::new(Mutex::new(Some(work_receiver))),
            result_sender: Arc::new(Mutex::new(Some(result_sender))),
            result_receiver: Arc::new(Mutex::new(Some(result_receiver))),
            event_sender,
            main_loop_handle: Arc::new(Mutex::new(None)),
            work_dispatch_handle: Arc::new(Mutex::new(None)),
            result_process_handle: Arc::new(Mutex::new(None)),
            hashmeter_update_handle: Arc::new(Mutex::new(None)),
            core_result_handle: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 根据配置的核心类型注册相应的设备驱动
    async fn register_drivers_for_cores(
        _device_manager: &mut DeviceManager,
        _cores_config: &crate::config::CoresConfig
    ) -> Result<(), MiningError> {
        debug!("Registering drivers for compiled core features");
        Ok(())
    }

    /// 创建挖矿核心
    pub async fn create_core(&self, core_type: &str, config: CoreConfig) -> Result<String, MiningError> {
        debug!("Creating mining core: {}", core_type);

        let core_id = self.core_registry.create_core(core_type, config).await
            .map_err(|e| MiningError::CoreError(format!("创建核心失败: {}", e)))?;

        debug!("Core created successfully: {}", core_id);
        Ok(core_id)
    }

    /// 列出可用的核心类型
    pub async fn list_available_cores(&self) -> Result<Vec<cgminer_core::CoreInfo>, MiningError> {
        self.core_registry.list_factories().await
            .map_err(|e| MiningError::CoreError(format!("获取核心列表失败: {}", e)))
    }

    /// 根据类型获取核心
    pub async fn get_cores_by_type(&self, core_type: &CoreType) -> Result<Vec<cgminer_core::CoreInfo>, MiningError> {
        self.core_registry.get_factories_by_type(core_type).await
            .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))
    }

    /// 移除挖矿核心
    pub async fn remove_core(&self, core_id: &str) -> Result<(), MiningError> {
        debug!("Removing mining core: {}", core_id);

        self.core_registry.remove_core(core_id).await
            .map_err(|e| MiningError::CoreError(format!("移除核心失败: {}", e)))?;

        debug!("Core removed successfully: {}", core_id);
        Ok(())
    }

    /// 注册核心（为示例程序提供接口）
    pub async fn register_core(&self, core_info: cgminer_core::CoreInfo) -> Result<String, MiningError> {
        debug!("Registering core: {}", core_info.name);

        // 简化实现：暂时返回成功
        let core_id = format!("core_{}", uuid::Uuid::new_v4());
        debug!("Core registered successfully: {}", core_id);
        Ok(core_id)
    }

        /// 提交工作（为示例程序提供接口）
    pub async fn submit_work_external(&self, work: cgminer_core::Work) -> Result<(), MiningError> {
        debug!("Submitting work: {}", work.job_id);

        // 直接使用cgminer-core的Work类型创建WorkItem
        let work_item = WorkItem::new(work);

        if let Ok(work_sender_guard) = self.work_sender.try_lock() {
            if let Some(sender) = work_sender_guard.as_ref() {
                sender.send(work_item)
                    .map_err(|e| MiningError::WorkError(format!("提交工作失败: {}", e)))?;
            }
        }

        Ok(())
    }

    /// 收集结果（为示例程序提供接口）
    pub async fn collect_results(&self) -> Result<Vec<cgminer_core::MiningResult>, MiningError> {
        // 简化实现：返回空结果
        Ok(vec![])
    }

    /// 启动挖矿
    pub async fn start(&self) -> Result<(), MiningError> {
        // 检查是否已经在运行
        if *self.running.read().await {
            warn!("cgminer already running");
            return Ok(());
        }

        // 更新状态
        *self.state.write().await = MiningState::Starting;
        *self.running.write().await = true;

        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Stopped,
            new_state: MiningState::Starting,
            timestamp: SystemTime::now(),
        }).await;

        // 启动核心组件
        let mut started_components = Vec::new();

        // 先启动挖矿核心（创建核心实例）
        self.start_cores().await?;
        started_components.push("cores");

        // 初始化设备管理器（使用协调器功能）
        self.initialize_device_manager().await?;
        started_components.push("devices");

        // 启动矿池管理器
        {
            let pool_manager = self.pool_manager.lock().await;
            pool_manager.start().await?;
            started_components.push("pools");
        }

        // 启动监控系统
        {
            let monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.start().await?;
            started_components.push("monitoring");
        }

                // 启动算力计量器
        self.start_hashmeter().await?;
        started_components.push("hashmeter");

        // 启动各个任务
        self.start_main_loop().await?;
        self.start_work_dispatch().await?;
        self.start_result_processing().await?;
        self.start_core_result_collection().await?;
        self.start_hashmeter_updates().await?;
        started_components.push("workers");

        // 更新状态和统计
        *self.state.write().await = MiningState::Running;
        self.stats.write().await.start();

        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Starting,
            new_state: MiningState::Running,
            timestamp: SystemTime::now(),
        }).await;

        // 等待设备完全启动后再显示总结
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 显示启动总结
        info!("Started cgminer {}", env!("CARGO_PKG_VERSION"));
        let first_pool = self.full_config.pools.pools.first();
        info!("Mining to {} with {} pools",
              first_pool.map(|p| p.url.as_str()).unwrap_or("unknown"),
              self.full_config.pools.pools.len());

        // 获取实际创建的设备数量（基于已编译的核心工厂）
        let actual_device_count = {
            let mut total_devices = 0u32;

            // 重新获取可用的核心工厂列表
            let available_factories = match self.core_registry.list_factories().await {
                Ok(factories) => factories,
                Err(_) => vec![], // 如果获取失败，使用空列表
            };

            // 基于可用的核心工厂来计算设备数量
            for factory_info in &available_factories {
                match factory_info.name.as_str() {
                    "Software Mining Core" => {
                        // CPU核心：使用配置的device_count
                        if let Some(cpu_btc_config) = &self.full_config.cores.cpu_btc {
                            total_devices += cpu_btc_config.device_count;
                        } else {
                            total_devices += 4; // 默认4个CPU设备
                        }
                    }
                    "Maijie L7 Core" => {
                        // ASIC核心：使用chains配置（只有在ASIC启用时）
                        if let Some(maijie_l7_config) = &self.full_config.cores.maijie_l7 {
                            if maijie_l7_config.enabled {
                                total_devices += self.full_config.devices.chains.len() as u32;
                            }
                        }
                    }
                    "GPU Mining Core Factory" => {
                        // GPU核心：使用配置的device_count或默认1个
                        if let Some(gpu_btc_config) = &self.full_config.cores.gpu_btc {
                            total_devices += gpu_btc_config.device_count;
                        } else {
                            total_devices += 1; // 默认1个GPU设备
                        }
                    }
                    _ => {
                        // 未知核心类型，默认1个设备
                        total_devices += 1;
                    }
                }
            }

            if total_devices == 0 {
                4 // 保底默认值
            } else {
                total_devices
            }
        };

        // 再次获取可用工厂数量用于显示
        let factories_count = match self.core_registry.list_factories().await {
            Ok(factories) => factories.len(),
            Err(_) => 0,
        };

        info!("Loaded {} compiled core factories, {} devices ready",
              factories_count,
              actual_device_count);
        Ok(())
    }

    /// 停止挖矿
    pub async fn stop(&self) -> Result<(), MiningError> {
        // 检查是否已经停止
        if !*self.running.read().await {
            warn!("cgminer already stopped");
            return Ok(());
        }

        // 更新状态
        *self.state.write().await = MiningState::Stopping;
        *self.running.write().await = false;

        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Running,
            new_state: MiningState::Stopping,
            timestamp: SystemTime::now(),
        }).await;

        // 停止各个任务
        self.stop_tasks().await;

        // 停止监控系统
        {
            let monitoring_system = self.monitoring_system.lock().await;
            monitoring_system.stop().await?;
        }

        // 停止矿池管理器
        {
            let pool_manager = self.pool_manager.lock().await;
            pool_manager.stop().await?;
        }

        // 停止设备管理器
        {
            let mut device_manager = self.device_manager.lock().await;
            device_manager.stop().await?;
        }

        // 更新状态
        *self.state.write().await = MiningState::Stopped;

        // 发送状态变更事件
        self.send_event(MiningEvent::StateChanged {
            old_state: MiningState::Stopping,
            new_state: MiningState::Stopped,
            timestamp: SystemTime::now(),
        }).await;

        // 显示停止总结
        let stats = self.get_stats().await;
        info!("Shutdown complete after {} runtime",
              format_duration(stats.uptime));
        info!("Summary: A:{} R:{} HW:{}",
              stats.accepted_shares,
              stats.rejected_shares,
              stats.hardware_errors);
        Ok(())
    }

    /// 获取挖矿状态
    pub async fn get_state(&self) -> MiningState {
        self.state.read().await.clone()
    }

    /// 获取挖矿统计
    pub async fn get_stats(&self) -> MiningStats {
        let mut stats = self.stats.write().await;
        stats.update_uptime();

        // 更新当前算力
        if let Ok(device_manager) = self.device_manager.try_lock() {
            let hashrate = device_manager.get_total_hashrate().await;
            stats.update_hashrate(hashrate);
        }

        stats.clone()
    }

    /// 获取系统状态
    pub async fn get_system_status(&self) -> SystemStatus {
        let stats = self.get_stats().await;
        let device_manager = self.device_manager.lock().await;
        let pool_manager = self.pool_manager.lock().await;

        SystemStatus {
            state: self.get_state().await,
            uptime: stats.uptime,
            total_hashrate: stats.current_hashrate,
            accepted_shares: stats.accepted_shares,
            rejected_shares: stats.rejected_shares,
            hardware_errors: stats.hardware_errors,
            active_devices: device_manager.get_active_device_count().await,
            connected_pools: pool_manager.get_connected_pool_count().await,
            current_difficulty: stats.current_difficulty,
            best_share: stats.best_share,
            efficiency: stats.efficiency,
            power_consumption: stats.power_consumption,
        }
    }

    /// 订阅事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<MiningEvent> {
        self.event_sender.subscribe()
    }

    /// 发送事件
    async fn send_event(&self, event: MiningEvent) {
        if let Err(e) = self.event_sender.send(event) {
            debug!("Failed to send mining event: {}", e);
        }
    }

    /// 启动主循环
    async fn start_main_loop(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let stats = self.stats.clone();
        let device_manager = self.device_manager.clone();
        let pool_manager = self.pool_manager.clone();
        let _monitoring_system = self.monitoring_system.clone();
        let _event_sender = self.event_sender.clone();
        let work_sender = self.work_sender.clone();
        let scan_interval = self.config.scan_interval;

        let handle = tokio::spawn(async move {
            let mut interval = interval(scan_interval);

            while *running.read().await {
                interval.tick().await;

                // 更新统计信息
                {
                    let mut stats = stats.write().await;
                    stats.update_uptime();

                    // 获取设备算力
                    if let Ok(device_manager) = device_manager.try_lock() {
                        let hashrate = device_manager.get_total_hashrate().await;
                        stats.update_hashrate(hashrate);
                    }
                }

                // 检查设备健康状态
                if let Ok(_device_manager) = device_manager.try_lock() {
                    // 这里可以添加设备健康检查逻辑
                }

                // 检查矿池连接状态并获取工作
                if let Ok(pool_manager) = pool_manager.try_lock() {
                    // 获取工作并发送到工作分发器
                    if let Ok(work_sender_guard) = work_sender.try_lock() {
                        if let Some(sender) = work_sender_guard.as_ref() {
                            // 尝试从矿池获取工作
                                                    match pool_manager.get_work().await {
                            Ok(work) => {
                                let work_item = WorkItem {
                                    work,
                                    assigned_device: None, // 让工作分发器决定分配给哪个设备
                                    created_at: SystemTime::now(),
                                    priority: 1,
                                    retry_count: 0,
                                };

                                if let Err(e) = sender.send(work_item) {
                                    debug!("Failed to send work to dispatcher: {}", e);
                                } else {
                                    debug!("Work sent to dispatcher");
                                }
                            }
                            Err(e) => {
                                debug!("Failed to get work from pool: {}", e);
                            }
                        }
                        }
                    }
                }
            }
        });

        *self.main_loop_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动统一工作分发器
    async fn start_work_dispatch(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let device_manager = self.device_manager.clone();
        let core_registry = self.core_registry.clone();
        let work_receiver = self.work_receiver.clone();

        let handle = tokio::spawn(async move {
            let receiver = work_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                debug!("Work dispatcher started");

                // 创建统一的工作分发器
                let work_dispatcher = UnifiedWorkDispatcher::new(
                    core_registry.clone(),
                    device_manager.clone(),
                );

                while *running.read().await {
                    match receiver.recv().await {
                        Some(work_item) => {
                            debug!("Received work item: {}", work_item.work.id);

                            // 使用统一的工作分发逻辑
                            match work_dispatcher.dispatch_work(work_item).await {
                                Ok(target) => {
                                    debug!("Work dispatched to: {}", target);
                                }
                                Err(e) => {
                                    debug!("Work dispatch failed: {}", e);
                                }
                            }
                        }
                        None => {
                            debug!("Work receiver closed");
                            break;
                        }
                    }
                }

                debug!("Work dispatcher stopped");
            } else {
                error!("Cannot get work receiver");
            }
        });

        *self.work_dispatch_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动结果处理
    async fn start_result_processing(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let pool_manager = self.pool_manager.clone();
        let stats = self.stats.clone();
        let result_receiver = self.result_receiver.clone();
        let event_sender = self.event_sender.clone();

        let handle = tokio::spawn(async move {
            let receiver = result_receiver.lock().await.take();
            if let Some(mut receiver) = receiver {
                while *running.read().await {
                    match receiver.recv().await {
                        Some(result_item) => {
                            // 处理挖矿结果
                            if result_item.is_valid() {
                                // 提交到矿池
                                if let Ok(_pool_manager) = pool_manager.try_lock() {
                                    // 这里需要实现份额提交逻辑
                                }

                                // 更新统计
                                {
                                    let mut stats = stats.write().await;
                                    stats.record_accepted_share(result_item.result.share_difficulty);
                                }

                                // 发送事件
                                let _ = event_sender.send(MiningEvent::ShareAccepted {
                                    work_id: result_item.result.work_id,
                                    difficulty: result_item.result.share_difficulty,
                                    timestamp: SystemTime::now(),
                                });
                            }
                        }
                        None => break,
                    }
                }
            }
        });

        *self.result_process_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动核心结果收集
    async fn start_core_result_collection(&self) -> Result<(), MiningError> {
        let running = self.running.clone();
        let core_registry = self.core_registry.clone();
        let _result_sender = self.result_sender.clone(); // 暂时不使用，因为我们不创建假的WorkItem
        let stats = self.stats.clone();
        let _pool_manager = self.pool_manager.clone(); // 暂时不使用，因为缺少工作数据
        let core_result_handle = self.core_result_handle.clone();
        let result_collection_interval = self.config.result_collection_interval;

        let handle = tokio::spawn(async move {
            // 确保间隔不为零，最小值为1毫秒
            let safe_interval = if result_collection_interval.is_zero() {
                Duration::from_millis(20) // 默认20毫秒
            } else {
                result_collection_interval
            };
            let mut interval = interval(safe_interval); // 使用安全的结果收集间隔

            while *running.read().await {
                interval.tick().await;

                // 从核心注册表获取所有活跃核心并收集结果
                match core_registry.list_active_cores().await {
                    Ok(active_core_ids) => {
                        if !active_core_ids.is_empty() {
                            debug!("Collecting results from {} active cores", active_core_ids.len());
                        }
                        for core_id in active_core_ids {
                            // 从核心注册表收集结果
                            match core_registry.collect_results_from_core(&core_id).await {
                                Ok(results) => {
                                    for core_result in results {
                                        // 转换核心结果到本地格式（work_id已经是UUID）
                                        let mut mining_result = cgminer_core::types::MiningResult::new(
                                            core_result.work_id,
                                            core_result.device_id,
                                            core_result.nonce,
                                            core_result.hash,
                                            core_result.meets_target,
                                        );

                                        // 设置extranonce2
                                        if core_result.extranonce2.len() >= 4 {
                                            mining_result = mining_result.with_extranonce2(core_result.extranonce2);
                                        }

                                        // 计算份额难度
                                        if let Err(e) = mining_result.calculate_share_difficulty() {
                                            warn!("Failed to calculate share difficulty: {}", e);
                                        }

                                                                // 处理真实挖矿结果
                        if core_result.meets_target {
                            info!("Valid share found from core {}, device {}", core_id, core_result.device_id);

                            // 记录找到的有效份额（只有真正找到时才记录）
                            {
                                let mut stats_guard = stats.write().await;
                                stats_guard.record_accepted_share(mining_result.share_difficulty);
                            }
                        }
                        // 注意：大部分哈希结果都不会满足目标难度，这是正常的
                        // 只有极少数结果会满足难度要求并成为有效份额
                                        }
                                }
                                Err(e) => {
                                    debug!("No results from core {}: {}", core_id, e);
                                }
                            }

                            // 获取核心的算力统计
                            match core_registry.get_core_stats(&core_id).await {
                                Ok(core_stats) => {
                                    // 更新总体算力统计
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.current_hashrate = core_stats.total_hashrate;
                                    stats_guard.average_hashrate = core_stats.average_hashrate;
                                }
                                Err(e) => {
                                    debug!("Failed to get stats from core {}: {}", core_id, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to list active cores: {}", e);
                    }
                }
            }
        });

        // 存储任务句柄
        *core_result_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 启动挖矿核心
    async fn start_cores(&self) -> Result<(), MiningError> {
        debug!("Starting mining cores");

        // 首先检查是否已经有活跃的核心（由设备管理器创建）
        match self.core_registry.list_active_cores().await {
            Ok(active_cores) => {
                if !active_cores.is_empty() {
                    debug!("Found {} mining core(s): {:?}", active_cores.len(), active_cores);

                    // 按照优先级选择最优核心：asic > gpu > cpu
                    let selected_core = self.select_optimal_core(&active_cores).await?;

                    info!("Selected optimal core: {} (priority: asic > gpu > cpu)", selected_core);

                    // 只启动选中的最优核心
                    match self.core_registry.start_core(&selected_core).await {
                        Ok(()) => {
                            info!("Started 1 mining core: {}", selected_core);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to start selected core {}: {}", selected_core, e);
                            return Err(MiningError::CoreError(format!("启动最优核心失败: {}", e)));
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to list active cores: {}", e);
            }
        }

        // 如果没有活跃的核心，则基于编译特性创建核心
        debug!("No active cores found, creating cores based on compiled features");

        // 获取编译时已注册的核心工厂列表
        let available_factories = match self.core_registry.list_factories().await {
            Ok(factories) => factories,
            Err(e) => {
                error!("Failed to list available core factories: {}", e);
                return Err(MiningError::CoreError(format!("获取可用核心工厂失败: {}", e)));
            }
        };

        info!("Found {} compiled core factories: {:?}",
              available_factories.len(),
              available_factories.iter().map(|f| &f.name).collect::<Vec<_>>());

        let mut created_cores = Vec::new();

        // 第一步：基于编译特性创建所有可用的核心（不立即启动）
        for factory_info in &available_factories {
            match factory_info.name.as_str() {
                "Software Mining Core" => {
                    debug!("Creating CPU BTC core");

                    // 创建软算法核心配置
                    let core_config = CoreConfig {
                        name: "software_core".to_string(),
                        enabled: true,
                        devices: vec![], // 设备配置将在核心内部创建
                        custom_params: {
                            let mut params = std::collections::HashMap::new();
                            if let Some(cpu_btc_config) = &self.full_config.cores.cpu_btc {
                                params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.device_count)));
                                params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.min_hashrate).unwrap()));
                                params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.max_hashrate).unwrap()));
                                params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(cpu_btc_config.error_rate).unwrap()));
                                params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.batch_size)));
                                params.insert("work_timeout_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(cpu_btc_config.work_timeout_ms)));
                            }
                            params
                        },
                    };

                    // 创建软算法核心（不启动）
                    let core_id = self.create_core("cpu-btc", core_config).await?;

                    // 检查核心是否创建成功
                    if self.core_registry.get_core(&core_id).await
                        .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))?.is_some() {
                        debug!("CPU BTC core created: {}", core_id);
                        created_cores.push(core_id);
                    }
                }
                "GPU Mining Core Factory" => {
                    debug!("Creating GPU BTC core");

                    // 创建GPU核心配置
                    let core_config = CoreConfig {
                        name: "gpu_core".to_string(),
                        enabled: true,
                        devices: vec![],
                        custom_params: {
                            let mut params = std::collections::HashMap::new();
                            if let Some(gpu_btc_config) = &self.full_config.cores.gpu_btc {
                                params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(gpu_btc_config.device_count)));
                                params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(gpu_btc_config.max_hashrate).unwrap()));
                                params.insert("work_size".to_string(), serde_json::Value::Number(serde_json::Number::from(gpu_btc_config.work_size)));
                                params.insert("work_timeout_ms".to_string(), serde_json::Value::Number(serde_json::Number::from(gpu_btc_config.work_timeout_ms)));

                                // 平台特定配置
                                #[cfg(target_os = "macos")]
                                {
                                    params.insert("backend".to_string(), serde_json::Value::String("metal".to_string()));
                                    params.insert("threads_per_threadgroup".to_string(), serde_json::Value::Number(serde_json::Number::from(512)));
                                }

                                #[cfg(not(target_os = "macos"))]
                                {
                                    params.insert("backend".to_string(), serde_json::Value::String("opencl".to_string()));
                                }
                            }
                            params
                        },
                    };

                    // 创建GPU核心（不启动）
                    let core_id = self.create_core("gpu-btc", core_config).await?;

                    // 检查核心是否创建成功
                    if self.core_registry.get_core(&core_id).await
                        .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))?.is_some() {
                        debug!("GPU BTC core created: {}", core_id);
                        created_cores.push(core_id);
                    }
                }
                "Maijie L7 Core" => {
                    if let Some(maijie_l7_config) = &self.full_config.cores.maijie_l7 {
                        if maijie_l7_config.enabled {
                            debug!("Creating Maijie L7 ASIC core");

                            let core_config = CoreConfig {
                                name: "maijie_l7_core".to_string(),
                                enabled: true,
                                devices: vec![], // 设备配置将在核心内部创建
                                custom_params: {
                                    let mut params = std::collections::HashMap::new();
                                    params.insert("chain_count".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.chain_count)));
                                    params.insert("spi_speed".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.spi_speed)));
                                    params.insert("uart_baud".to_string(), serde_json::Value::Number(serde_json::Number::from(maijie_l7_config.uart_baud)));
                                    params.insert("auto_detect".to_string(), serde_json::Value::Bool(maijie_l7_config.auto_detect));
                                    params.insert("power_limit".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(maijie_l7_config.power_limit).unwrap()));
                                    params.insert("cooling_mode".to_string(), serde_json::Value::String(maijie_l7_config.cooling_mode.clone()));
                                    params
                                },
                            };

                            // 创建ASIC核心（不启动）
                            let core_id = self.create_core("maijie-l7", core_config).await?;

                            if self.core_registry.get_core(&core_id).await
                                .map_err(|e| MiningError::CoreError(format!("获取核心失败: {}", e)))?.is_some() {
                                debug!("ASIC core created: {}", core_id);
                                created_cores.push(core_id);
                            }
                        }
                    }
                }
                _ => {
                    debug!("Unknown core factory: {}", factory_info.name);
                }
            }
        }

        // 第二步：如果创建了多个核心，使用优先级选择最优核心启动
        if !created_cores.is_empty() {
            info!("Created {} mining cores", created_cores.len());

            if created_cores.len() == 1 {
                // 只有一个核心，直接启动
                let core_id = &created_cores[0];
                match self.core_registry.start_core(core_id).await {
                    Ok(()) => {
                        info!("Started mining core: {}", core_id);
                    }
                    Err(e) => {
                        error!("Failed to start core {}: {}", core_id, e);
                        return Err(MiningError::CoreError(format!("启动核心失败: {}", e)));
                    }
                }
            } else {
                // 多个核心，使用优先级选择
                let selected_core = self.select_optimal_core(&created_cores).await?;

                info!("Selected optimal core: {} (priority: asic > gpu > cpu)", selected_core);

                match self.core_registry.start_core(&selected_core).await {
                    Ok(()) => {
                        info!("Started optimal mining core: {}", selected_core);
                    }
                    Err(e) => {
                        error!("Failed to start selected core {}: {}", selected_core, e);
                        return Err(MiningError::CoreError(format!("启动最优核心失败: {}", e)));
                    }
                }
            }
        } else {
            warn!("No mining cores were created");
        }

        Ok(())
    }

    /// 按照优先级选择最优核心：asic > gpu > cpu
    async fn select_optimal_core(&self, active_cores: &[String]) -> Result<String, MiningError> {
        debug!("Selecting optimal core from {} candidates", active_cores.len());

        // 定义核心类型优先级（数字越小优先级越高）
        let get_core_priority = |core_id: &str| -> u8 {
            if core_id.contains("asic") || core_id.contains("maijie") || core_id.contains("l7") {
                1 // ASIC 最高优先级
            } else if core_id.contains("gpu") {
                2 // GPU 中等优先级 - 必须在CPU判断之前！
            } else if core_id.contains("cpu") || core_id.contains("software") {
                3 // CPU 最低优先级 - 移除了"btc"关键字避免与GPU冲突
            } else {
                4 // 未知类型，最低优先级
            }
        };

        // 按优先级排序核心
        let mut sorted_cores: Vec<(String, u8)> = active_cores
            .iter()
            .map(|core_id| (core_id.clone(), get_core_priority(core_id)))
            .collect();

        sorted_cores.sort_by_key(|(_, priority)| *priority);

        // 输出优先级信息
        for (core_id, priority) in &sorted_cores {
            let core_type = match priority {
                1 => "ASIC",
                2 => "GPU",
                3 => "CPU",
                _ => "Unknown",
            };
            debug!("Core: {} -> Type: {} (Priority: {})", core_id, core_type, priority);
        }

        // 选择最高优先级的核心
        if let Some((selected_core, priority)) = sorted_cores.first() {
            let core_type = match priority {
                1 => "ASIC",
                2 => "GPU",
                3 => "CPU",
                _ => "Unknown",
            };
            info!("Selected {} core: {} (highest priority)", core_type, selected_core);
            Ok(selected_core.clone())
        } else {
            Err(MiningError::CoreError("No cores available for selection".to_string()))
        }
    }

    /// 启动算力计量器
    async fn start_hashmeter(&self) -> Result<(), MiningError> {
        let hashmeter_guard = self.hashmeter.lock().await;
        if let Some(hashmeter) = hashmeter_guard.as_ref() {
            hashmeter.start().await?;
            debug!("Hashmeter started");
        }
        Ok(())
    }

    /// 启动算力数据更新任务
    async fn start_hashmeter_updates(&self) -> Result<(), MiningError> {
        let hashmeter = self.hashmeter.clone();
        let stats = self.stats.clone();
        let device_manager = self.device_manager.clone();
        let _monitoring_system = self.monitoring_system.clone();
        let running = self.running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // 每5秒更新一次数据

            while *running.read().await {
                interval.tick().await;

                // 检查是否有hashmeter
                let hashmeter_guard = hashmeter.lock().await;
                if let Some(hashmeter) = hashmeter_guard.as_ref() {
                    // 获取挖矿统计数据
                    let stats_guard = stats.read().await;

                    // 获取活跃设备数量（从设备管理器获取真实数量）
                    let active_devices = if let Ok(device_mgr) = device_manager.try_lock() {
                        device_mgr.get_active_device_count().await
                    } else {
                        0 // 如果设备管理器被锁定，显示0
                    };

                    // 获取连接的矿池数量
                    let connected_pools = 1; // 暂时固定为1，表示有活跃的矿池连接

                    let mining_metrics = MiningMetrics {
                        timestamp: SystemTime::now(),
                        total_hashrate: stats_guard.current_hashrate,
                        accepted_shares: stats_guard.accepted_shares,
                        rejected_shares: stats_guard.rejected_shares,
                        hardware_errors: stats_guard.hardware_errors,
                        stale_shares: stats_guard.stale_shares,
                        best_share: stats_guard.best_share,
                        current_difficulty: stats_guard.current_difficulty,
                        network_difficulty: stats_guard.network_difficulty,
                        blocks_found: stats_guard.blocks_found,
                        efficiency: stats_guard.efficiency,
                        active_devices,
                        connected_pools,
                    };

                    // 更新总体统计
                    if let Err(e) = hashmeter.update_total_stats(&mining_metrics).await {
                        warn!("Failed to update hashmeter total stats: {}", e);
                    }

                    // 更新设备级统计数据 - 从设备管理器获取真实的设备统计
                    if let Ok(device_mgr) = device_manager.try_lock() {
                        // 获取所有设备信息
                        let device_infos = device_mgr.get_all_device_info().await;

                        // 为每个设备更新统计信息
                        for device_info in device_infos {
                            // 尝试获取设备的核心统计信息
                            if let Ok(device_stats_core) = device_mgr.get_device_stats_core(device_info.id).await {
                                // 更新设备统计到算力计量器
                                if let Err(e) = hashmeter.update_device_stats(&device_stats_core).await {
                                    debug!("Failed to update device {} stats: {}", device_info.id, e);
                                }
                            }
                        }
                    }
                }
            }
        });

        *self.hashmeter_update_handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止所有任务
    async fn stop_tasks(&self) {
        // 停止算力计量器
        {
            let hashmeter_guard = self.hashmeter.lock().await;
            if let Some(hashmeter) = hashmeter_guard.as_ref() {
                if let Err(e) = hashmeter.stop().await {
                    warn!("Failed to stop hashmeter: {}", e);
                }
            }
        }

        // 停止算力更新任务
        if let Some(handle) = self.hashmeter_update_handle.lock().await.take() {
            handle.abort();
        }

        // 停止主循环
        if let Some(handle) = self.main_loop_handle.lock().await.take() {
            handle.abort();
        }

        // 停止工作分发
        if let Some(handle) = self.work_dispatch_handle.lock().await.take() {
            handle.abort();
        }

        // 停止结果处理
        if let Some(handle) = self.result_process_handle.lock().await.take() {
            handle.abort();
        }

        // 停止核心结果收集
        if let Some(handle) = self.core_result_handle.lock().await.take() {
            handle.abort();
        }
    }

    /// 初始化设备管理器（从协调器移植）
    async fn initialize_device_manager(&self) -> Result<(), MiningError> {
        debug!("Initializing device manager");

        let active_core_ids = self.core_registry.list_active_cores().await
            .map_err(|e| MiningError::CoreError(format!("获取活跃核心列表失败: {}", e)))?;

        let mut device_manager = self.device_manager.lock().await;
        device_manager.set_active_cores(active_core_ids).await;
        device_manager.initialize().await?;
        device_manager.start().await?;

        // 验证设备映射
        device_manager.validate_device_mappings().await?;

        debug!("Device manager initialized");
        Ok(())
    }

    /// 提交工作（从协调器移植）
    pub async fn submit_work(&self, work: crate::device::Work) -> Result<(), MiningError> {
        let work_item = WorkItem {
            work,
            assigned_device: None,
            created_at: SystemTime::now(),
            priority: 1,
            retry_count: 0,
        };

        if let Ok(work_sender_guard) = self.work_sender.try_lock() {
            if let Some(sender) = work_sender_guard.as_ref() {
                sender.send(work_item)
                    .map_err(|e| MiningError::WorkError(format!("提交工作失败: {}", e)))?;
            }
        }

        Ok(())
    }

    /// 获取设备-核心映射器
    pub fn get_device_core_mapper(&self) -> Arc<DeviceCoreMapper> {
        self.device_core_mapper.clone()
    }
}

/// 系统状态
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub state: MiningState,
    pub uptime: Duration,
    pub total_hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub hardware_errors: u64,
    pub active_devices: u32,
    pub connected_pools: u32,
    pub current_difficulty: f64,
    pub best_share: f64,
    pub efficiency: f64,
    pub power_consumption: f64,
}

/// 统一工作分发器
/// 负责将工作统一分发到核心或设备，避免分发逻辑的重复和不一致
pub struct UnifiedWorkDispatcher {
    core_registry: Arc<CoreRegistry>,
    device_manager: Arc<Mutex<DeviceManager>>,
}

impl UnifiedWorkDispatcher {
    /// 创建新的统一工作分发器
    pub fn new(
        core_registry: Arc<CoreRegistry>,
        device_manager: Arc<Mutex<DeviceManager>>,
    ) -> Self {
        Self {
            core_registry,
            device_manager,
        }
    }

    /// 分发工作
    /// 优先级：活跃核心 > 指定设备 > 任意可用设备
    pub async fn dispatch_work(&self, work_item: WorkItem) -> Result<String, String> {
        debug!("Dispatching work: {}", work_item.work.id);

        // 1. 优先尝试分发到活跃的核心
        match self.dispatch_to_cores(&work_item).await {
            Ok(target) => {
                debug!("Work dispatched to: {}", target);
                return Ok(target);
            }
            Err(e) => {
                debug!("Core dispatch failed: {}", e);
            }
        }

        // 2. 如果核心分发失败，尝试分发到设备
        match self.dispatch_to_devices(&work_item).await {
            Ok(target) => {
                debug!("Work dispatched to: {}", target);
                return Ok(target);
            }
            Err(e) => {
                debug!("Device dispatch failed: {}", e);
            }
        }

        debug!("Work dispatch failed: no available targets");
        Err("No available cores or devices for work dispatch".to_string())
    }

    /// 分发工作到核心
    async fn dispatch_to_cores(&self, work_item: &WorkItem) -> Result<String, String> {
        debug!("Dispatching work to cores");

        let active_core_ids = self.core_registry.list_active_cores().await
            .map_err(|e| format!("Failed to list active cores: {}", e))?;

        debug!("Found {} active cores", active_core_ids.len());

        if active_core_ids.is_empty() {
            return Err("No active cores available".to_string());
        }

        // 使用轮询策略分发到核心
        for core_id in &active_core_ids {
            debug!("Trying to submit work to core: {}", core_id);
            match self.core_registry.submit_work_to_core(core_id, work_item.work.clone().into()).await {
                Ok(()) => {
                    debug!("Work submitted to core: {}", core_id);
                    return Ok(format!("core:{}", core_id));
                }
                Err(e) => {
                    debug!("Failed to submit work to core {}: {}", core_id, e);
                    continue;
                }
            }
        }

        debug!("All cores rejected the work");
        Err("All cores rejected the work".to_string())
    }

    /// 分发工作到设备
    async fn dispatch_to_devices(&self, work_item: &WorkItem) -> Result<String, String> {
        let device_manager = self.device_manager.try_lock()
            .map_err(|_| "Device manager is busy".to_string())?;

        // 如果指定了设备，优先分发到该设备
        if let Some(device_id) = work_item.assigned_device {
            match device_manager.submit_work(device_id, work_item.work.clone()).await {
                Ok(()) => {
                    return Ok(format!("device:{}", device_id));
                }
                Err(e) => {
                    debug!("Failed to submit work to assigned device {}: {}", device_id, e);
                }
            }
        }

        // 如果没有指定设备或指定设备失败，尝试分发到任意可用设备
        // 这里需要从设备管理器获取可用设备列表
        // 由于当前DeviceManager没有提供获取所有设备的方法，我们暂时返回错误
        Err("No available devices for work dispatch".to_string())
    }
}
