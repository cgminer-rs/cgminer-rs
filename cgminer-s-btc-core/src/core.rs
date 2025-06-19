//! 软算法挖矿核心实现

use cgminer_core::{
    MiningCore, CoreInfo, CoreCapabilities, CoreConfig, CoreStats, CoreError,
    DeviceInfo, MiningDevice, Work, MiningResult
};
use crate::device::SoftwareDevice;
use crate::performance::PerformanceOptimizer;
use crate::cpu_affinity::{CpuAffinityManager, CpuAffinityStrategy};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

/// 软算法挖矿核心
pub struct SoftwareMiningCore {
    /// 核心信息
    core_info: CoreInfo,
    /// 核心能力
    capabilities: CoreCapabilities,
    /// 核心配置
    config: Option<CoreConfig>,
    /// 设备列表
    devices: Arc<Mutex<HashMap<u32, Box<dyn MiningDevice>>>>,
    /// 核心统计信息
    stats: Arc<RwLock<CoreStats>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 启动时间
    start_time: Option<SystemTime>,
    /// 性能优化器
    performance_optimizer: Option<PerformanceOptimizer>,
    /// CPU绑定管理器
    cpu_affinity_manager: Option<Arc<RwLock<CpuAffinityManager>>>,
}

impl SoftwareMiningCore {
    /// 创建新的软算法挖矿核心
    pub fn new(name: String) -> Self {
        let core_info = CoreInfo::new(
            name.clone(),
            cgminer_core::CoreType::Custom("software".to_string()),
            crate::VERSION.to_string(),
            "软算法挖矿核心，使用真实的SHA256算法进行CPU挖矿计算".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["software".to_string(), "cpu".to_string()],
        );

        let capabilities = CoreCapabilities {
            supports_auto_tuning: false,
            supports_temperature_monitoring: true,
            supports_voltage_control: false,
            supports_frequency_control: true,
            supports_fan_control: false,
            supports_multiple_chains: true,
            max_devices: Some(64), // 软算法核心支持最多64个设备
            supported_algorithms: vec!["SHA256".to_string(), "SHA256d".to_string()],
        };

        let stats = CoreStats::new(name);

        Self {
            core_info,
            capabilities,
            config: None,
            devices: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            running: Arc::new(RwLock::new(false)),
            start_time: None,
            performance_optimizer: None,
            cpu_affinity_manager: None,
        }
    }

    /// 创建软算法设备
    async fn create_software_devices(&self, config: &CoreConfig) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        let mut devices = Vec::new();

        // 从配置中获取设备数量
        let device_count = config.custom_params
            .get("device_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as u32;

        // 获取算力范围
        let min_hashrate = config.custom_params
            .get("min_hashrate")
            .and_then(|v| v.as_f64())
            .unwrap_or(1_000_000_000.0); // 1 GH/s

        let max_hashrate = config.custom_params
            .get("max_hashrate")
            .and_then(|v| v.as_f64())
            .unwrap_or(5_000_000_000.0); // 5 GH/s

        let error_rate = config.custom_params
            .get("error_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.01); // 1%

        let batch_size = config.custom_params
            .get("batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as u32;

        info!("创建 {} 个软算法设备，算力范围: {:.2} - {:.2} GH/s",
              device_count,
              min_hashrate / 1_000_000_000.0,
              max_hashrate / 1_000_000_000.0);

        for i in 0..device_count {
            // 为每个设备分配不同的算力
            let device_hashrate = min_hashrate +
                (max_hashrate - min_hashrate) * (i as f64 / device_count.max(1) as f64);

            let mut device_config = if (i as usize) < config.devices.len() {
                config.devices[i as usize].clone()
            } else {
                cgminer_core::DeviceConfig {
                    chain_id: i as u8,
                    enabled: true,
                    frequency: 600 + (i * 50), // 递增频率
                    voltage: 900 + (i * 20),   // 递增电压
                    auto_tune: false,
                    chip_count: 64,
                    temperature_limit: 80.0,
                    fan_speed: Some(50 + i * 5),
                }
            };

            // 应用性能优化
            if let Some(optimizer) = &self.performance_optimizer {
                optimizer.apply_to_device_config(&mut device_config, 1000 + i);
            }

            let device_info = DeviceInfo::new(
                1000 + i,
                format!("Software Device {}", i),
                "software".to_string(),
                i as u8,
            );

            let device = if let Some(cpu_affinity) = &self.cpu_affinity_manager {
                // 为CPU绑定管理器分配设备
                {
                    let mut affinity_manager = cpu_affinity.write().map_err(|e| {
                        CoreError::runtime(format!("Failed to acquire write lock: {}", e))
                    })?;
                    affinity_manager.assign_cpu_core(1000 + i);
                }

                SoftwareDevice::new_with_cpu_affinity(
                    device_info,
                    device_config,
                    device_hashrate,
                    error_rate,
                    batch_size,
                    cpu_affinity.clone(),
                ).await?
            } else {
                SoftwareDevice::new(
                    device_info,
                    device_config,
                    device_hashrate,
                    error_rate,
                    batch_size,
                ).await?
            };

            devices.push(Box::new(device) as Box<dyn MiningDevice>);
        }

        Ok(devices)
    }

    /// 更新统计信息
    async fn update_stats(&self) -> Result<(), CoreError> {
        let devices = self.devices.lock().await;
        let mut total_hashrate = 0.0;
        let mut total_accepted = 0;
        let mut total_rejected = 0;
        let mut total_errors = 0;
        let mut active_devices = 0;

        for device in devices.values() {
            if let Ok(stats) = device.get_stats().await {
                total_hashrate += stats.current_hashrate.hashes_per_second;
                total_accepted += stats.accepted_work;
                total_rejected += stats.rejected_work;
                total_errors += stats.hardware_errors;
                active_devices += 1;
            }
        }

        let mut stats = self.stats.write().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire write lock: {}", e))
        })?;

        stats.device_count = devices.len() as u32;
        stats.active_devices = active_devices;
        stats.total_hashrate = total_hashrate;
        stats.average_hashrate = if active_devices > 0 {
            total_hashrate / active_devices as f64
        } else {
            0.0
        };
        stats.accepted_work = total_accepted;
        stats.rejected_work = total_rejected;
        stats.hardware_errors = total_errors;

        if let Some(start_time) = self.start_time {
            stats.uptime = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::from_secs(0));
        }

        stats.last_updated = SystemTime::now();

        Ok(())
    }
}

#[async_trait]
impl MiningCore for SoftwareMiningCore {
    /// 获取核心信息
    fn get_info(&self) -> &CoreInfo {
        &self.core_info
    }

    /// 获取核心能力
    fn get_capabilities(&self) -> &CoreCapabilities {
        &self.capabilities
    }

    /// 初始化核心
    async fn initialize(&mut self, config: CoreConfig) -> Result<(), CoreError> {
        println!("🔧 [CORE DEBUG] 开始初始化软算法挖矿核心: {}", config.name);
        println!("📋 [CORE DEBUG] 配置参数: {:?}", config.custom_params);
        error!("🔧 [DEBUG] 开始初始化软算法挖矿核心: {}", config.name);
        error!("📋 [DEBUG] 配置参数: {:?}", config.custom_params);

        // 验证配置
        println!("✅ [CORE DEBUG] 验证配置...");
        error!("✅ [DEBUG] 验证配置...");
        self.validate_config(&config)?;
        println!("✅ [CORE DEBUG] 配置验证通过");
        error!("✅ [DEBUG] 配置验证通过");

        // 初始化性能优化器
        let mut perf_config = crate::performance::PerformanceConfig::default();
        let mut optimizer = PerformanceOptimizer::new(perf_config.clone());
        optimizer.optimize_for_system();
        perf_config = optimizer.get_config().clone();
        self.performance_optimizer = Some(optimizer);

        // 初始化CPU绑定管理器
        if perf_config.cpu_affinity.enabled {
            let strategy = match perf_config.cpu_affinity.strategy.as_str() {
                "round_robin" => CpuAffinityStrategy::RoundRobin,
                "intelligent" => CpuAffinityStrategy::Intelligent,
                "load_balanced" => CpuAffinityStrategy::LoadBalanced,
                "physical_cores_only" => CpuAffinityStrategy::PhysicalCoresOnly,
                "performance_first" => CpuAffinityStrategy::PerformanceFirst,
                _ => CpuAffinityStrategy::Intelligent,
            };

            let cpu_manager = CpuAffinityManager::new(true, strategy);
            self.cpu_affinity_manager = Some(Arc::new(RwLock::new(cpu_manager)));
            info!("✅ CPU绑定管理器已启用，策略: {}", perf_config.cpu_affinity.strategy);
        }

        // 创建设备
        println!("🏭 [CORE DEBUG] 开始创建软算法设备...");
        error!("🏭 [DEBUG] 开始创建软算法设备...");
        let devices = self.create_software_devices(&config).await?;
        println!("✅ [CORE DEBUG] 软算法设备创建完成，共创建 {} 个设备", devices.len());
        error!("✅ [DEBUG] 软算法设备创建完成，共创建 {} 个设备", devices.len());

        // 存储设备
        {
            let mut device_map = self.devices.lock().await;
            for device in devices {
                let device_id = device.device_id();
                device_map.insert(device_id, device);
            }
        }

        // 初始化所有设备
        {
            let mut device_map = self.devices.lock().await;
            for (device_id, device) in device_map.iter_mut() {
                let device_config = config.devices
                    .iter()
                    .find(|dc| dc.chain_id == (*device_id - 1000) as u8)
                    .cloned()
                    .unwrap_or_default();

                if let Err(e) = device.initialize(device_config).await {
                    error!("初始化设备 {} 失败: {}", device_id, e);
                    return Err(CoreError::Device(e));
                }
            }
        }

        self.config = Some(config);

        // 检查设备数量
        let device_count = {
            let devices = self.devices.lock().await;
            devices.len()
        };
        println!("📊 [CORE DEBUG] 最终设备数量: {}", device_count);
        error!("📊 [DEBUG] 最终设备数量: {}", device_count);

        println!("🎉 [CORE DEBUG] 软算法挖矿核心初始化完成");
        error!("🎉 [DEBUG] 软算法挖矿核心初始化完成");
        Ok(())
    }

    /// 启动核心
    async fn start(&mut self) -> Result<(), CoreError> {
        info!("启动软算法挖矿核心");

        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;

            if *running {
                return Err(CoreError::runtime("核心已经在运行中"));
            }
            *running = true;
        }

        // 启动所有设备
        {
            let mut devices = self.devices.lock().await;
            for (device_id, device) in devices.iter_mut() {
                if let Err(e) = device.start().await {
                    error!("启动设备 {} 失败: {}", device_id, e);
                    // 继续启动其他设备，不因为一个设备失败而停止
                }
            }
        }

        self.start_time = Some(SystemTime::now());
        info!("软算法挖矿核心启动完成");
        Ok(())
    }

    /// 停止核心
    async fn stop(&mut self) -> Result<(), CoreError> {
        info!("停止软算法挖矿核心");

        {
            let mut running = self.running.write().map_err(|e| {
                CoreError::runtime(format!("Failed to acquire write lock: {}", e))
            })?;
            *running = false;
        }

        // 停止所有设备
        {
            let mut devices = self.devices.lock().await;
            for (device_id, device) in devices.iter_mut() {
                if let Err(e) = device.stop().await {
                    error!("停止设备 {} 失败: {}", device_id, e);
                }
            }
        }

        info!("软算法挖矿核心已停止");
        Ok(())
    }

    /// 重启核心
    async fn restart(&mut self) -> Result<(), CoreError> {
        info!("重启软算法挖矿核心");
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.start().await?;
        Ok(())
    }

    /// 扫描设备
    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, CoreError> {
        debug!("扫描软算法设备");

        let devices = self.devices.lock().await;
        let mut device_infos = Vec::new();

        for device in devices.values() {
            match device.get_info().await {
                Ok(info) => device_infos.push(info),
                Err(e) => warn!("获取设备信息失败: {}", e),
            }
        }

        Ok(device_infos)
    }

    /// 创建设备
    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, CoreError> {
        info!("创建软算法设备: {}", device_info.name);

        let device_config = cgminer_core::DeviceConfig::default();

        // 从配置中获取参数，如果没有配置则使用合理的默认值
        let default_config = CoreConfig::default();
        let config = self.config.as_ref().unwrap_or(&default_config);

        let target_hashrate = config.custom_params
            .get("max_hashrate")
            .and_then(|v| v.as_f64())
            .unwrap_or(2_000_000_000.0); // 2 GH/s 默认算力

        let error_rate = config.custom_params
            .get("error_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.01); // 1% 错误率

        let batch_size = config.custom_params
            .get("batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as u32; // 批次大小

        let device = SoftwareDevice::new(
            device_info,
            device_config,
            target_hashrate,
            error_rate,
            batch_size,
        ).await?;

        Ok(Box::new(device))
    }

    /// 获取所有设备
    async fn get_devices(&self) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        Err(CoreError::runtime("get_devices 方法暂未实现"))
    }

    /// 获取设备数量
    async fn device_count(&self) -> Result<u32, CoreError> {
        let devices = self.devices.lock().await;
        Ok(devices.len() as u32)
    }

    /// 提交工作到所有设备
    async fn submit_work(&mut self, work: Work) -> Result<(), CoreError> {
        println!("🚀 [CORE WORK] 软算法核心接收到工作: {}", work.id);
        debug!("提交工作到所有软算法设备: {}", work.id);

        let mut devices = self.devices.lock().await;
        println!("📊 [CORE WORK] 向 {} 个软算法设备分发工作", devices.len());

        for (device_id, device) in devices.iter_mut() {
            match device.submit_work(work.clone()).await {
                Ok(()) => {
                    println!("✅ [CORE WORK] 工作成功提交到设备 {}", device_id);
                }
                Err(e) => {
                    println!("❌ [CORE WORK] 向设备 {} 提交工作失败: {}", device_id, e);
                    warn!("向设备 {} 提交工作失败: {}", device_id, e);
                }
            }
        }

        println!("🎯 [CORE WORK] 工作分发完成");
        Ok(())
    }

    /// 收集所有设备的挖矿结果
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>, CoreError> {
        let mut results = Vec::new();
        let mut devices = self.devices.lock().await;

        println!("🔍 [CORE RESULTS] 开始收集 {} 个设备的挖矿结果", devices.len());

        for (device_id, device) in devices.iter_mut() {
            match device.get_result().await {
                Ok(Some(result)) => {
                    println!("💎 [CORE RESULTS] 设备 {} 产生挖矿结果: nonce={:08x}", device_id, result.nonce);
                    results.push(result);
                }
                Ok(None) => {
                    // 没有结果 - 这是正常的
                },
                Err(e) => {
                    println!("❌ [CORE RESULTS] 获取设备 {} 挖矿结果失败: {}", device_id, e);
                    warn!("获取设备挖矿结果失败: {}", e);
                }
            }
        }

        println!("📊 [CORE RESULTS] 收集完成，共收集到 {} 个挖矿结果", results.len());
        debug!("收集到 {} 个挖矿结果", results.len());
        Ok(results)
    }

    /// 获取核心统计信息
    async fn get_stats(&self) -> Result<CoreStats, CoreError> {
        self.update_stats().await?;
        let stats = self.stats.read().map_err(|e| {
            CoreError::runtime(format!("Failed to acquire read lock: {}", e))
        })?;
        Ok(stats.clone())
    }

    /// 健康检查
    async fn health_check(&self) -> Result<bool, CoreError> {
        let devices = self.devices.lock().await;
        let mut healthy_devices = 0;

        for device in devices.values() {
            match device.health_check().await {
                Ok(true) => healthy_devices += 1,
                Ok(false) => {},
                Err(e) => warn!("设备健康检查失败: {}", e),
            }
        }

        // 如果至少有一半设备健康，则认为核心健康
        let health_threshold = (devices.len() + 1) / 2;
        Ok(healthy_devices >= health_threshold)
    }

    /// 验证配置
    fn validate_config(&self, config: &CoreConfig) -> Result<(), CoreError> {
        if config.name.is_empty() {
            return Err(CoreError::config("核心名称不能为空"));
        }

        // 验证设备数量
        if let Some(device_count) = config.custom_params.get("device_count") {
            if let Some(count) = device_count.as_u64() {
                if count == 0 {
                    return Err(CoreError::config("软算法设备数量不能为0"));
                }
                if count > 64 {
                    return Err(CoreError::config("软算法设备数量不能超过64"));
                }
            }
        }

        Ok(())
    }

    /// 获取默认配置
    fn default_config(&self) -> CoreConfig {
        use std::collections::HashMap;

        let mut custom_params = HashMap::new();
        custom_params.insert("device_count".to_string(), serde_json::Value::Number(serde_json::Number::from(4)));
        custom_params.insert("min_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(1_000_000_000.0).unwrap()));
        custom_params.insert("max_hashrate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(5_000_000_000.0).unwrap()));
        custom_params.insert("error_rate".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap()));
        custom_params.insert("batch_size".to_string(), serde_json::Value::Number(serde_json::Number::from(1000)));

        CoreConfig {
            name: "software-core".to_string(),
            enabled: true,
            devices: vec![cgminer_core::DeviceConfig::default(); 4],
            custom_params,
        }
    }

    /// 关闭核心
    async fn shutdown(&mut self) -> Result<(), CoreError> {
        info!("关闭软算法挖矿核心");
        self.stop().await?;

        // 清空设备列表
        {
            let mut devices = self.devices.lock().await;
            devices.clear();
        }

        info!("软算法挖矿核心已关闭");
        Ok(())
    }
}
