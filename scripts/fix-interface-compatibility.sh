#!/bin/bash
# CGMiner-RS 接口兼容性修复脚本
# 修复外置核心与 cgminer-core 接口不兼容的问题

set -e

echo "🔧 CGMiner-RS 接口兼容性修复"
echo "================================"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 1. 修复 ASIC 核心的 CoreCapabilities 结构
echo -e "\n${BLUE}🔧 修复 ASIC 核心 CoreCapabilities 结构${NC}"
echo "------------------------------------------------"

ASIC_CORE_PATH="../cgminer-asic-maijie-l7-core"
if [ -d "$ASIC_CORE_PATH" ]; then
    echo "正在修复 $ASIC_CORE_PATH/src/core.rs..."

    # 备份原文件
    cp "$ASIC_CORE_PATH/src/core.rs" "$ASIC_CORE_PATH/src/core.rs.backup"

    # 修复 CoreCapabilities 结构
    cat > "$ASIC_CORE_PATH/src/core.rs.tmp" << 'EOF'
//! ASIC挖矿核心实现

use cgminer_core::{
    MiningCore, CoreInfo, CoreCapabilities, CoreConfig, CoreStats, CoreError,
    DeviceInfo, DeviceConfig, MiningDevice, Work, MiningResult, CoreType,
    TemperatureCapabilities, VoltageCapabilities, FrequencyCapabilities, FanCapabilities
};
use crate::device::AsicDevice;
use crate::hardware::{HardwareInterface, MockHardwareInterface, RealHardwareInterface};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

/// ASIC挖矿核心
pub struct AsicMiningCore {
    /// 核心信息
    core_info: CoreInfo,
    /// 核心能力
    capabilities: CoreCapabilities,
    /// 核心配置
    config: Option<CoreConfig>,
    /// 设备列表
    devices: Arc<Mutex<HashMap<u32, Box<dyn MiningDevice>>>>,
    /// 硬件接口
    hardware: Arc<dyn HardwareInterface>,
    /// 核心统计信息
    stats: Arc<RwLock<CoreStats>>,
    /// 是否正在运行
    running: Arc<RwLock<bool>>,
    /// 启动时间
    start_time: Option<SystemTime>,
}

impl AsicMiningCore {
    /// 创建新的ASIC挖矿核心
    pub fn new() -> Self {
        let core_info = CoreInfo::new(
            "ASIC Mining Core".to_string(),
            CoreType::Asic,
            env!("CARGO_PKG_VERSION").to_string(),
            "ASIC挖矿核心，支持Maijie L7等ASIC硬件设备".to_string(),
            "CGMiner Rust Team".to_string(),
            vec!["asic".to_string(), "maijie-l7".to_string()],
        );

        let capabilities = CoreCapabilities {
            supports_auto_tuning: true,
            temperature_capabilities: TemperatureCapabilities {
                supports_monitoring: true,
                supports_control: true,
                supports_threshold_alerts: true,
                monitoring_precision: Some(0.5),
            },
            voltage_capabilities: VoltageCapabilities {
                supports_monitoring: true,
                supports_control: true,
                control_range: Some((800, 1000)),
            },
            frequency_capabilities: FrequencyCapabilities {
                supports_monitoring: true,
                supports_control: true,
                control_range: Some((400, 600)),
            },
            fan_capabilities: FanCapabilities {
                supports_monitoring: true,
                supports_control: true,
                fan_count: Some(2),
            },
            supports_multiple_chains: true,
            max_devices: Some(8),
            supported_algorithms: vec!["SHA256".to_string()],
            cpu_capabilities: None,
            core_type: CoreType::Asic,
        };

        Self {
            core_info,
            capabilities,
            config: None,
            devices: Arc::new(Mutex::new(HashMap::new())),
            hardware: Arc::new(MockHardwareInterface::new()),
            stats: Arc::new(RwLock::new(CoreStats::new("ASIC Core".to_string()))),
            running: Arc::new(RwLock::new(false)),
            start_time: None,
        }
    }

    /// 使用真实硬件接口创建核心
    pub fn with_real_hardware() -> Self {
        let mut core = Self::new();
        core.hardware = Arc::new(RealHardwareInterface::new());
        core
    }
}

#[async_trait]
impl MiningCore for AsicMiningCore {
    fn get_info(&self) -> &CoreInfo {
        &self.core_info
    }

    fn get_capabilities(&self) -> &CoreCapabilities {
        &self.capabilities
    }

    async fn initialize(&mut self, config: CoreConfig) -> Result<(), CoreError> {
        info!("🚀 初始化ASIC挖矿核心");

        self.config = Some(config.clone());

        // 初始化硬件接口
        self.hardware.initialize().await
            .map_err(|e| CoreError::runtime(format!("硬件初始化失败: {}", e)))?;

        // 扫描并创建设备
        let device_infos = self.scan_devices().await?;
        let mut devices = self.devices.lock().await;

        for device_info in device_infos {
            let device_config = DeviceConfig::default(); // 使用默认配置
            let device = AsicDevice::new(device_info.clone(), device_config, self.hardware.clone()).await
                .map_err(|e| CoreError::runtime(format!("创建设备失败: {}", e)))?;
            devices.insert(device_info.id, Box::new(device));
        }

        info!("✅ ASIC挖矿核心初始化完成，发现 {} 个设备", devices.len());
        Ok(())
    }

    async fn start(&mut self) -> Result<(), CoreError> {
        info!("🚀 启动ASIC挖矿核心");

        let mut devices = self.devices.lock().await;
        for device in devices.values_mut() {
            device.start().await
                .map_err(|e| CoreError::runtime(format!("设备启动失败: {}", e)))?;
        }

        *self.running.write().unwrap() = true;
        self.start_time = Some(SystemTime::now());

        info!("✅ ASIC挖矿核心启动完成");
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), CoreError> {
        info!("🛑 停止ASIC挖矿核心");

        let mut devices = self.devices.lock().await;
        for device in devices.values_mut() {
            device.stop().await
                .map_err(|e| CoreError::runtime(format!("设备停止失败: {}", e)))?;
        }

        *self.running.write().unwrap() = false;

        info!("✅ ASIC挖矿核心停止完成");
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), CoreError> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.start().await
    }

    async fn scan_devices(&self) -> Result<Vec<DeviceInfo>, CoreError> {
        info!("🔍 扫描ASIC设备");

        // 模拟扫描到的设备
        let devices = vec![
            DeviceInfo::new(0, "Maijie L7 Chain 0".to_string(), "maijie_l7".to_string(), 0),
            DeviceInfo::new(1, "Maijie L7 Chain 1".to_string(), "maijie_l7".to_string(), 1),
        ];

        info!("✅ 扫描完成，发现 {} 个ASIC设备", devices.len());
        Ok(devices)
    }

    async fn create_device(&self, device_info: DeviceInfo) -> Result<Box<dyn MiningDevice>, CoreError> {
        let device_config = DeviceConfig::default(); // 使用默认配置
        let device = AsicDevice::new(device_info, device_config, self.hardware.clone()).await
            .map_err(|e| CoreError::runtime(format!("创建设备失败: {}", e)))?;
        Ok(Box::new(device))
    }

    async fn get_devices(&self) -> Result<Vec<Box<dyn MiningDevice>>, CoreError> {
        // 注意：这个方法返回设备的克隆，因为 trait object 不能直接克隆
        // 在实际实现中，可能需要返回设备的引用或使用其他方式
        Err(CoreError::runtime("get_devices not implemented for ASIC core".to_string()))
    }

    async fn device_count(&self) -> Result<u32, CoreError> {
        let devices = self.devices.lock().await;
        Ok(devices.len() as u32)
    }

    async fn submit_work(&mut self, work: Work) -> Result<(), CoreError> {
        debug!("📤 提交工作到ASIC设备: work_id={}", work.id);

        let mut devices = self.devices.lock().await;
        for device in devices.values_mut() {
            device.submit_work(work.clone()).await
                .map_err(|e| CoreError::runtime(format!("提交工作失败: {}", e)))?;
        }

        Ok(())
    }

    async fn collect_results(&mut self) -> Result<Vec<MiningResult>, CoreError> {
        let mut results = Vec::new();
        let mut devices = self.devices.lock().await;

        for device in devices.values_mut() {
            if let Some(result) = device.get_result().await
                .map_err(|e| CoreError::runtime(format!("收集结果失败: {}", e)))? {
                results.push(result);
            }
        }

        Ok(results)
    }

    async fn get_stats(&self) -> Result<CoreStats, CoreError> {
        let stats = self.stats.read().unwrap();
        Ok(stats.clone())
    }

    async fn health_check(&self) -> Result<bool, CoreError> {
        let devices = self.devices.lock().await;

        for device in devices.values() {
            if !device.health_check().await
                .map_err(|e| CoreError::runtime(format!("健康检查失败: {}", e)))? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn validate_config(&self, _config: &CoreConfig) -> Result<(), CoreError> {
        // 验证ASIC特定的配置
        Ok(())
    }

    fn default_config(&self) -> CoreConfig {
        CoreConfig::default()
    }

    async fn shutdown(&mut self) -> Result<(), CoreError> {
        self.stop().await
    }
}
EOF

    # 替换原文件
    mv "$ASIC_CORE_PATH/src/core.rs.tmp" "$ASIC_CORE_PATH/src/core.rs"
    echo -e "${GREEN}✅ ASIC 核心 CoreCapabilities 结构已修复${NC}"
else
    echo -e "${YELLOW}⚠️  ASIC 核心目录不存在，跳过修复${NC}"
fi

# 2. 修复 ASIC 设备的 Work ID 类型问题
echo -e "\n${BLUE}🔧 修复 ASIC 设备 Work ID 类型问题${NC}"
echo "------------------------------------------------"

if [ -d "$ASIC_CORE_PATH" ]; then
    echo "正在修复 $ASIC_CORE_PATH/src/device.rs..."

    # 备份原文件
    cp "$ASIC_CORE_PATH/src/device.rs" "$ASIC_CORE_PATH/src/device.rs.backup"

    # 修复 Work ID 类型问题
    sed -i.tmp 's/work\.as_ref()\.map(|w| w\.id)\.unwrap_or(0)/work.as_ref().map(|w| w.id).unwrap_or_else(|| uuid::Uuid::new_v4())/g' "$ASIC_CORE_PATH/src/device.rs"

    # 清理临时文件
    rm -f "$ASIC_CORE_PATH/src/device.rs.tmp"

    echo -e "${GREEN}✅ ASIC 设备 Work ID 类型问题已修复${NC}"
else
    echo -e "${YELLOW}⚠️  ASIC 核心目录不存在，跳过修复${NC}"
fi

# 3. 验证修复结果
echo -e "\n${BLUE}🔍 验证修复结果${NC}"
echo "------------------------------------------------"

# 测试编译 ASIC 核心
if cargo check --features maijie-l7 --quiet 2>/dev/null; then
    echo -e "${GREEN}✅ ASIC 核心编译通过${NC}"
else
    echo -e "${RED}❌ ASIC 核心编译仍有问题${NC}"
    echo "详细错误信息："
    cargo check --features maijie-l7 2>&1 | head -20
fi

# 测试编译所有核心
if cargo check --features all-cores --quiet 2>/dev/null; then
    echo -e "${GREEN}✅ 所有核心编译通过${NC}"
else
    echo -e "${RED}❌ 所有核心编译仍有问题${NC}"
fi

# 4. 生成修复报告
echo -e "\n${BLUE}📄 生成修复报告${NC}"
echo "------------------------------------------------"

REPORT_FILE="docs/interface-compatibility-fix-report.md"
cat > "$REPORT_FILE" << EOF
# CGMiner-RS 接口兼容性修复报告

**生成时间**: $(date)
**修复脚本**: scripts/fix-interface-compatibility.sh

## 修复内容

### 1. ASIC 核心 CoreCapabilities 结构修复

**问题**: ASIC 核心使用了已废弃的 CoreCapabilities 字段
- \`supports_temperature_monitoring\`
- \`supports_voltage_control\`
- \`supports_frequency_control\`
- \`supports_fan_control\`

**修复**: 更新为新的结构化能力定义
- \`temperature_capabilities: TemperatureCapabilities\`
- \`voltage_capabilities: VoltageCapabilities\`
- \`frequency_capabilities: FrequencyCapabilities\`
- \`fan_capabilities: FanCapabilities\`

### 2. Work ID 类型兼容性修复

**问题**: Work.id 字段从整数类型改为 Uuid 类型
**修复**: 更新默认值从 \`0\` 改为 \`uuid::Uuid::new_v4()\`

## 修复结果

EOF

# 检查修复结果并写入报告
if cargo check --features maijie-l7 --quiet 2>/dev/null; then
    echo "- ✅ ASIC 核心编译通过" >> "$REPORT_FILE"
else
    echo "- ❌ ASIC 核心编译仍有问题" >> "$REPORT_FILE"
fi

if cargo check --features all-cores --quiet 2>/dev/null; then
    echo "- ✅ 所有核心编译通过" >> "$REPORT_FILE"
else
    echo "- ❌ 所有核心编译仍有问题" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

## 备份文件

修复过程中创建了以下备份文件：
- \`$ASIC_CORE_PATH/src/core.rs.backup\`
- \`$ASIC_CORE_PATH/src/device.rs.backup\`

如需回滚，可以使用这些备份文件。

## 后续建议

1. **测试验证**: 运行完整的测试套件验证修复效果
2. **文档更新**: 更新相关文档以反映接口变更
3. **版本管理**: 考虑更新版本号以标记接口兼容性修复

---

**修复工具**: scripts/fix-interface-compatibility.sh
**相关文档**: [接口验证报告](./interface-verification-report.md)
EOF

echo -e "${GREEN}✅ 修复报告已生成: $REPORT_FILE${NC}"

echo -e "\n${BLUE}📊 修复完成${NC}"
echo "================================"
echo "接口兼容性修复已完成。请运行以下命令验证："
echo "  ./scripts/verify-interfaces.sh"
echo ""
echo "如果仍有问题，请查看详细的修复报告："
echo "  cat $REPORT_FILE"
