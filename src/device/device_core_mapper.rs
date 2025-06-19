//! 设备-核心映射管理器
//!
//! 负责管理设备与挖矿核心之间的映射关系，解决设备管理和核心管理的架构问题

use crate::error::DeviceError;
use cgminer_core::{CoreInfo, DeviceInfo, CoreRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// 设备-核心映射信息
#[derive(Debug, Clone)]
pub struct DeviceCoreMapping {
    /// 设备ID
    pub device_id: u32,
    /// 核心名称
    pub core_name: String,
    /// 核心类型
    pub core_type: String,
    /// 设备在核心中的索引
    pub device_index: u32,
    /// 映射创建时间
    pub created_at: std::time::SystemTime,
    /// 是否活跃
    pub active: bool,
}

/// 设备-核心映射管理器
pub struct DeviceCoreMapper {
    /// 设备到核心的映射
    device_to_core: Arc<RwLock<HashMap<u32, DeviceCoreMapping>>>,
    /// 核心到设备列表的映射
    core_to_devices: Arc<RwLock<HashMap<String, Vec<u32>>>>,
    /// 设备ID分配器
    device_id_allocator: Arc<RwLock<DeviceIdAllocator>>,
    /// 核心注册表
    core_registry: Arc<CoreRegistry>,
}

/// 设备ID分配器
#[derive(Debug)]
struct DeviceIdAllocator {
    /// 下一个可用的设备ID
    next_id: u32,
    /// 已分配的设备ID
    allocated_ids: std::collections::HashSet<u32>,
    /// 核心类型到ID范围的映射
    core_type_ranges: HashMap<String, (u32, u32)>, // (start, end)
}

impl DeviceIdAllocator {
    fn new() -> Self {
        let mut core_type_ranges = HashMap::new();

        // 为不同核心类型分配ID范围，避免冲突
        core_type_ranges.insert("software".to_string(), (1000, 1499));
        core_type_ranges.insert("btc-software".to_string(), (1000, 1499));
        core_type_ranges.insert("btc".to_string(), (1000, 1499));
        core_type_ranges.insert("cpu".to_string(), (1000, 1499));
        core_type_ranges.insert("asic".to_string(), (2000, 2499));
        core_type_ranges.insert("maijie-l7".to_string(), (2000, 2499));
        core_type_ranges.insert("l7".to_string(), (2000, 2499));
        core_type_ranges.insert("fpga".to_string(), (3000, 3499));
        core_type_ranges.insert("gpu".to_string(), (4000, 4499));

        Self {
            next_id: 1000,
            allocated_ids: std::collections::HashSet::new(),
            core_type_ranges,
        }
    }

    /// 为指定核心类型分配设备ID
    fn allocate_id(&mut self, core_type: &str) -> Result<u32, DeviceError> {
        let (start, end) = self.core_type_ranges
            .get(core_type)
            .copied()
            .unwrap_or((1000, 1999)); // 默认范围

        // 在指定范围内查找可用ID
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

    /// 释放设备ID
    fn deallocate_id(&mut self, device_id: u32) {
        self.allocated_ids.remove(&device_id);
    }

    /// 检查ID是否已分配
    fn is_allocated(&self, device_id: u32) -> bool {
        self.allocated_ids.contains(&device_id)
    }
}

impl DeviceCoreMapper {
    /// 创建新的设备-核心映射管理器
    pub fn new(core_registry: Arc<CoreRegistry>) -> Self {
        Self {
            device_to_core: Arc::new(RwLock::new(HashMap::new())),
            core_to_devices: Arc::new(RwLock::new(HashMap::new())),
            device_id_allocator: Arc::new(RwLock::new(DeviceIdAllocator::new())),
            core_registry,
        }
    }

    /// 为核心创建设备映射
    pub async fn create_device_mappings_for_core(
        &self,
        core_info: &CoreInfo,
        device_infos: Vec<DeviceInfo>,
    ) -> Result<Vec<DeviceCoreMapping>, DeviceError> {
        let mut mappings = Vec::new();
        let mut device_to_core = self.device_to_core.write().await;
        let mut core_to_devices = self.core_to_devices.write().await;
        let mut allocator = self.device_id_allocator.write().await;

        // 获取或创建核心的设备列表
        let core_devices = core_to_devices
            .entry(core_info.name.clone())
            .or_insert_with(Vec::new);

        for (index, _device_info) in device_infos.into_iter().enumerate() {
            // 分配新的设备ID
            let device_id = allocator.allocate_id(&core_info.core_type.to_string())?;

            let mapping = DeviceCoreMapping {
                device_id,
                core_name: core_info.name.clone(),
                core_type: core_info.core_type.to_string(),
                device_index: index as u32,
                created_at: std::time::SystemTime::now(),
                active: true,
            };

            // 更新映射
            device_to_core.insert(device_id, mapping.clone());
            core_devices.push(device_id);
            mappings.push(mapping);

            info!(
                "创建设备映射: 设备ID={}, 核心={}, 类型={}, 索引={}",
                device_id, core_info.name, core_info.core_type, index
            );
        }

        Ok(mappings)
    }

    /// 获取设备的核心映射
    pub async fn get_device_mapping(&self, device_id: u32) -> Option<DeviceCoreMapping> {
        let device_to_core = self.device_to_core.read().await;
        device_to_core.get(&device_id).cloned()
    }

    /// 获取核心的所有设备ID
    pub async fn get_core_devices(&self, core_name: &str) -> Vec<u32> {
        let core_to_devices = self.core_to_devices.read().await;
        core_to_devices.get(core_name).cloned().unwrap_or_default()
    }

    /// 移除设备映射
    pub async fn remove_device_mapping(&self, device_id: u32) -> Result<(), DeviceError> {
        let mut device_to_core = self.device_to_core.write().await;
        let mut core_to_devices = self.core_to_devices.write().await;
        let mut allocator = self.device_id_allocator.write().await;

        if let Some(mapping) = device_to_core.remove(&device_id) {
            // 从核心的设备列表中移除
            if let Some(devices) = core_to_devices.get_mut(&mapping.core_name) {
                devices.retain(|&id| id != device_id);
            }

            // 释放设备ID
            allocator.deallocate_id(device_id);

            info!("移除设备映射: 设备ID={}, 核心={}", device_id, mapping.core_name);
            Ok(())
        } else {
            Err(DeviceError::NotFound { device_id })
        }
    }

    /// 获取所有映射
    pub async fn get_all_mappings(&self) -> HashMap<u32, DeviceCoreMapping> {
        let device_to_core = self.device_to_core.read().await;
        device_to_core.clone()
    }

    /// 获取映射统计信息
    pub async fn get_mapping_stats(&self) -> MappingStats {
        let device_to_core = self.device_to_core.read().await;
        let core_to_devices = self.core_to_devices.read().await;

        let total_devices = device_to_core.len();
        let total_cores = core_to_devices.len();
        let active_devices = device_to_core.values().filter(|m| m.active).count();

        // 按核心类型统计
        let mut devices_by_core_type = HashMap::new();
        for mapping in device_to_core.values() {
            *devices_by_core_type.entry(mapping.core_type.clone()).or_insert(0) += 1;
        }

        MappingStats {
            total_devices,
            total_cores,
            active_devices,
            devices_by_core_type,
        }
    }

    /// 验证映射一致性
    pub async fn validate_mappings(&self) -> Result<(), DeviceError> {
        let device_to_core = self.device_to_core.read().await;
        let core_to_devices = self.core_to_devices.read().await;

        // 检查双向映射一致性
        for (device_id, mapping) in device_to_core.iter() {
            if let Some(devices) = core_to_devices.get(&mapping.core_name) {
                if !devices.contains(device_id) {
                    return Err(DeviceError::InvalidState {
                        device_id: *device_id,
                        state: format!("Device {} not found in core {} device list", device_id, mapping.core_name),
                    });
                }
            } else {
                return Err(DeviceError::InvalidState {
                    device_id: *device_id,
                    state: format!("Core {} not found for device {}", mapping.core_name, device_id),
                });
            }
        }

        info!("设备映射验证通过");
        Ok(())
    }
}

/// 映射统计信息
#[derive(Debug, Clone)]
pub struct MappingStats {
    pub total_devices: usize,
    pub total_cores: usize,
    pub active_devices: usize,
    pub devices_by_core_type: HashMap<String, usize>,
}
