//! 设备管理架构优化模块
//!
//! 解决4设备vs32设备配置的架构问题，统一设备管理和核心管理的关系

use crate::error::{DeviceError, MiningError};
use crate::config::{DeviceConfig, CoresConfig};
use cgminer_core::{CoreInfo, CoreRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// 设备架构配置
#[derive(Debug, Clone)]
pub struct DeviceArchitectureConfig {
    /// 每个核心的最大设备数
    pub max_devices_per_core: u32,
    /// 设备ID分配策略
    pub device_id_strategy: DeviceIdStrategy,
    /// 设备扩展策略
    pub scaling_strategy: ScalingStrategy,
    /// 资源限制
    pub resource_limits: ResourceLimits,
}

/// 设备ID分配策略
#[derive(Debug, Clone)]
pub enum DeviceIdStrategy {
    /// 连续分配（1, 2, 3, ...）
    Sequential,
    /// 按核心类型分段分配
    SegmentedByCore,
    /// 按设备类型分段分配
    SegmentedByDevice,
}

/// 设备扩展策略
#[derive(Debug, Clone)]
pub enum ScalingStrategy {
    /// 固定数量（不支持动态扩展）
    Fixed,
    /// 动态扩展（根据需要增加设备）
    Dynamic { min_devices: u32, max_devices: u32 },
    /// 自适应扩展（根据系统资源自动调整）
    Adaptive { target_cpu_usage: f64 },
}

/// 资源限制配置
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// 最大内存使用（MB）
    pub max_memory_mb: u64,
    /// 最大CPU使用率（百分比）
    pub max_cpu_percent: f64,
    /// 最大设备数量
    pub max_total_devices: u32,
    /// 每个核心的最大设备数
    pub max_devices_per_core: u32,
}

impl Default for DeviceArchitectureConfig {
    fn default() -> Self {
        Self {
            max_devices_per_core: 64, // 增加到64以支持更多软算法设备
            device_id_strategy: DeviceIdStrategy::SegmentedByCore,
            scaling_strategy: ScalingStrategy::Dynamic {
                min_devices: 1,
                max_devices: 64 // 增加到64
            },
            resource_limits: ResourceLimits {
                max_memory_mb: 4096, // 增加内存限制以支持更多设备
                max_cpu_percent: 90.0,
                max_total_devices: 128, // 增加总设备限制
                max_devices_per_core: 64, // 增加每核心设备限制
            },
        }
    }
}

/// 统一设备架构管理器
pub struct UnifiedDeviceArchitecture {
    /// 架构配置
    config: DeviceArchitectureConfig,
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
    /// 核心到设备数量的映射
    core_device_counts: Arc<RwLock<HashMap<String, u32>>>,
    /// 设备ID分配器
    device_id_allocator: Arc<RwLock<DeviceIdAllocator>>,
    /// 资源监控器
    resource_monitor: Arc<RwLock<ResourceMonitor>>,
}

/// 设备ID分配器（改进版）
#[derive(Debug)]
struct DeviceIdAllocator {
    /// 下一个可用ID
    next_id: u32,
    /// 已分配的ID
    allocated_ids: std::collections::HashSet<u32>,
    /// 核心类型到ID范围的映射
    core_type_ranges: HashMap<String, (u32, u32)>,
    /// 分配策略
    strategy: DeviceIdStrategy,
}

/// 资源监控器
#[derive(Debug, Default)]
struct ResourceMonitor {
    /// 当前内存使用（MB）
    current_memory_mb: u64,
    /// 当前CPU使用率
    current_cpu_percent: f64,
    /// 当前设备总数
    current_device_count: u32,
    /// 每个核心的设备数量
    devices_per_core: HashMap<String, u32>,
}

impl UnifiedDeviceArchitecture {
    /// 创建新的统一设备架构管理器
    pub fn new(
        config: DeviceArchitectureConfig,
        core_registry: Arc<CoreRegistry>,
    ) -> Self {
        let device_id_allocator = DeviceIdAllocator::new(config.device_id_strategy.clone());

        Self {
            config,
            core_registry,
            core_device_counts: Arc::new(RwLock::new(HashMap::new())),
            device_id_allocator: Arc::new(RwLock::new(device_id_allocator)),
            resource_monitor: Arc::new(RwLock::new(ResourceMonitor::default())),
        }
    }

    /// 从配置创建架构管理器
    pub fn from_config(
        device_config: &DeviceConfig,
        cores_config: &CoresConfig,
        core_registry: Arc<CoreRegistry>,
    ) -> Result<Self, MiningError> {
        let arch_config = Self::derive_architecture_config(device_config, cores_config)?;
        Ok(Self::new(arch_config, core_registry))
    }

    /// 从设备和核心配置推导架构配置
    fn derive_architecture_config(
        _device_config: &DeviceConfig,
        cores_config: &CoresConfig,
    ) -> Result<DeviceArchitectureConfig, MiningError> {
        let mut max_devices = 4u32; // 默认值

        // 从核心配置中推导设备数量
        if cores_config.enabled_cores.contains(&"software".to_string()) {
            // 软算法核心通常支持更多设备，增加到64以支持高性能挖矿
            max_devices = 64;
        }

        // 检查是否有明确的设备数量配置
        // 这里需要根据实际的配置结构来调整

        let scaling_strategy = if max_devices <= 4 {
            ScalingStrategy::Fixed
        } else {
            ScalingStrategy::Dynamic {
                min_devices: 1,
                max_devices
            }
        };

        Ok(DeviceArchitectureConfig {
            max_devices_per_core: max_devices,
            device_id_strategy: DeviceIdStrategy::SegmentedByCore,
            scaling_strategy,
            resource_limits: ResourceLimits {
                max_memory_mb: if max_devices > 32 { 8192 } else if max_devices > 16 { 4096 } else { 2048 },
                max_cpu_percent: 90.0,
                max_total_devices: max_devices * 2, // 允许一些余量
                max_devices_per_core: max_devices,
            },
        })
    }

    /// 验证设备配置的一致性
    pub async fn validate_device_configuration(
        &self,
        core_info: &CoreInfo,
        requested_device_count: u32,
    ) -> Result<u32, DeviceError> {
        debug!("验证设备配置: 核心={}, 请求设备数={}", core_info.name, requested_device_count);

        // 1. 检查是否超过核心限制
        if requested_device_count > self.config.max_devices_per_core {
            return Err(DeviceError::InvalidConfig {
                reason: format!(
                    "Requested device count {} exceeds max devices per core {}",
                    requested_device_count, self.config.max_devices_per_core
                ),
            });
        }

        // 2. 检查资源限制
        let resource_monitor = self.resource_monitor.read().await;
        let total_devices_after = resource_monitor.current_device_count + requested_device_count;

        if total_devices_after > self.config.resource_limits.max_total_devices {
            return Err(DeviceError::InvalidConfig {
                reason: format!(
                    "Total device count {} would exceed system limit {}",
                    total_devices_after, self.config.resource_limits.max_total_devices
                ),
            });
        }

        // 3. 根据扩展策略调整设备数量
        let adjusted_count = match &self.config.scaling_strategy {
            ScalingStrategy::Fixed => {
                // 固定策略：使用配置的最大设备数
                std::cmp::min(requested_device_count, self.config.max_devices_per_core)
            }
            ScalingStrategy::Dynamic { min_devices, max_devices } => {
                // 动态策略：在范围内调整
                std::cmp::max(*min_devices, std::cmp::min(requested_device_count, *max_devices))
            }
            ScalingStrategy::Adaptive { target_cpu_usage: _ } => {
                // 自适应策略：根据当前系统负载调整
                self.calculate_adaptive_device_count(requested_device_count).await
            }
        };

        info!("设备配置验证通过: 核心={}, 调整后设备数={}", core_info.name, adjusted_count);
        Ok(adjusted_count)
    }

    /// 计算自适应设备数量
    async fn calculate_adaptive_device_count(&self, requested: u32) -> u32 {
        let resource_monitor = self.resource_monitor.read().await;

        // 简单的自适应算法：根据当前CPU使用率调整
        let cpu_factor = if resource_monitor.current_cpu_percent > 80.0 {
            0.5 // 高CPU使用率时减少设备数
        } else if resource_monitor.current_cpu_percent < 50.0 {
            1.2 // 低CPU使用率时可以增加设备数
        } else {
            1.0 // 正常使用率
        };

        let adjusted = (requested as f64 * cpu_factor) as u32;
        std::cmp::max(1, std::cmp::min(adjusted, self.config.max_devices_per_core))
    }

    /// 分配设备ID
    pub async fn allocate_device_ids(
        &self,
        core_info: &CoreInfo,
        device_count: u32,
    ) -> Result<Vec<u32>, DeviceError> {
        let mut allocator = self.device_id_allocator.write().await;
        let mut device_ids = Vec::new();

        for _ in 0..device_count {
            let device_id = allocator.allocate_id(&core_info.core_type.to_string())?;
            device_ids.push(device_id);
        }

        // 更新核心设备计数
        {
            let mut counts = self.core_device_counts.write().await;
            counts.insert(core_info.name.clone(), device_count);
        }

        // 更新资源监控
        {
            let mut monitor = self.resource_monitor.write().await;
            monitor.current_device_count += device_count;
            monitor.devices_per_core.insert(core_info.name.clone(), device_count);
        }

        info!("为核心 {} 分配了 {} 个设备ID: {:?}", core_info.name, device_count, device_ids);
        Ok(device_ids)
    }

    /// 释放设备ID
    pub async fn deallocate_device_ids(
        &self,
        core_name: &str,
        device_ids: &[u32],
    ) -> Result<(), DeviceError> {
        let mut allocator = self.device_id_allocator.write().await;

        for &device_id in device_ids {
            allocator.deallocate_id(device_id);
        }

        // 更新核心设备计数
        {
            let mut counts = self.core_device_counts.write().await;
            counts.remove(core_name);
        }

        // 更新资源监控
        {
            let mut monitor = self.resource_monitor.write().await;
            monitor.current_device_count = monitor.current_device_count.saturating_sub(device_ids.len() as u32);
            monitor.devices_per_core.remove(core_name);
        }

        info!("释放了核心 {} 的 {} 个设备ID", core_name, device_ids.len());
        Ok(())
    }

    /// 获取架构统计信息
    pub async fn get_architecture_stats(&self) -> ArchitectureStats {
        let monitor = self.resource_monitor.read().await;
        let counts = self.core_device_counts.read().await;

        ArchitectureStats {
            total_devices: monitor.current_device_count,
            devices_per_core: counts.clone(),
            memory_usage_mb: monitor.current_memory_mb,
            cpu_usage_percent: monitor.current_cpu_percent,
            max_devices_per_core: self.config.max_devices_per_core,
            scaling_strategy: self.config.scaling_strategy.clone(),
        }
    }

    /// 更新资源使用情况
    pub async fn update_resource_usage(&self, memory_mb: u64, cpu_percent: f64) {
        let mut monitor = self.resource_monitor.write().await;
        monitor.current_memory_mb = memory_mb;
        monitor.current_cpu_percent = cpu_percent;
    }
}

impl DeviceIdAllocator {
    fn new(strategy: DeviceIdStrategy) -> Self {
        let mut core_type_ranges = HashMap::new();

        // 根据策略设置ID范围
        match strategy {
            DeviceIdStrategy::Sequential => {
                // 所有设备使用连续ID
                core_type_ranges.insert("*".to_string(), (1, 10000));
            }
            DeviceIdStrategy::SegmentedByCore => {
                // 按核心类型分段
                core_type_ranges.insert("software".to_string(), (1000, 1999));
                core_type_ranges.insert("cpu-btc".to_string(), (1000, 1999));
                core_type_ranges.insert("asic".to_string(), (2000, 2999));
                core_type_ranges.insert("maijie-l7".to_string(), (2000, 2999));
            }
            DeviceIdStrategy::SegmentedByDevice => {
                // 按设备类型分段
                core_type_ranges.insert("cpu".to_string(), (1000, 1499));
                core_type_ranges.insert("asic".to_string(), (2000, 2499));
                core_type_ranges.insert("fpga".to_string(), (3000, 3499));
            }
        }

        Self {
            next_id: 1000,
            allocated_ids: std::collections::HashSet::new(),
            core_type_ranges,
            strategy,
        }
    }

    fn allocate_id(&mut self, core_type: &str) -> Result<u32, DeviceError> {
        let (start, end) = match &self.strategy {
            DeviceIdStrategy::Sequential => (1, 10000),
            _ => self.core_type_ranges
                .get(core_type)
                .copied()
                .unwrap_or((1000, 1999)),
        };

        for id in start..=end {
            if !self.allocated_ids.contains(&id) {
                self.allocated_ids.insert(id);
                return Ok(id);
            }
        }

        Err(DeviceError::InitializationFailed {
            device_id: 0,
            reason: format!("No available device ID in range {}-{} for core type {}", start, end, core_type),
        })
    }

    fn deallocate_id(&mut self, device_id: u32) {
        self.allocated_ids.remove(&device_id);
    }
}

/// 架构统计信息
#[derive(Debug, Clone)]
pub struct ArchitectureStats {
    pub total_devices: u32,
    pub devices_per_core: HashMap<String, u32>,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub max_devices_per_core: u32,
    pub scaling_strategy: ScalingStrategy,
}
