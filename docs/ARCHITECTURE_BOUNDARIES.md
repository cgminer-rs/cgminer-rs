# CGMiner-RS 架构边界定义

## 📖 概述

本文档明确定义了 CGMiner-RS 生态系统中应用层与引擎层的职责边界，确保清晰的架构分离和高效的协作。

## 🏗️ 整体架构

```text
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (cgminer-rs)                      │
│  🎯 应用编排 | 🌐 网络服务 | 📊 监控管理 | ⚙️ 配置管理       │
└─────────────────────────────────────────────────────────────┘
                                │
                        标准化接口 (cgminer-core)
                                │
┌─────────────────────────────────────────────────────────────┐
│                 引擎层 (外置核心模块)                       │
│  ⚡ 挖矿算法 | 🔧 设备控制 | 📈 性能优化 | 🌡️ 硬件监控     │
│                                                              │
│  • cgminer-cpu-btc-core     • cgminer-gpu-btc-core         │
│  • cgminer-asic-*-core      • 其他专用核心...               │
└─────────────────────────────────────────────────────────────┘
                                │
                          硬件抽象层
                                │
┌─────────────────────────────────────────────────────────────┐
│                    物理硬件层                               │
│     💻 CPU | 🎮 GPU | 🔌 ASIC | 🌡️ 传感器 | ⚡ 电源        │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 应用层职责边界 (cgminer-rs)

### ✅ 核心职责

#### 1. **应用程序入口和生命周期管理**
- 主程序启动、初始化、关闭流程
- 信号处理 (SIGTERM, SIGINT 等)
- 优雅关闭和资源清理
- 错误恢复和重启机制

#### 2. **配置管理**
- 配置文件解析 (TOML, JSON 等)
- 命令行参数处理
- 环境变量集成
- 配置验证和默认值处理
- 动态配置热重载

#### 3. **矿池连接和网络管理**
- Stratum 协议实现
- 矿池连接池管理
- 网络故障转移
- SOCKS5 代理支持
- SSL/TLS 连接管理

#### 4. **工作分发和结果收集**
- 从矿池接收工作 (getwork, stratum)
- 工作任务路由到合适的引擎核心
- 收集各核心的挖矿结果
- 结果验证和提交到矿池

#### 5. **API 服务和 Web 界面**
- RESTful API 服务器
- WebSocket 实时数据推送
- Web 管理界面
- 第三方集成接口

#### 6. **监控和日志管理**
- 系统级监控 (CPU、内存、网络)
- 日志收集、格式化、轮转
- 性能指标聚合
- 告警系统
- Prometheus/Grafana 集成

#### 7. **核心编排和调度**
- 挖矿核心实例管理
- 负载均衡策略
- 故障检测和核心切换
- 资源分配优化

### ❌ **不负责的领域**
- 具体挖矿算法实现
- 硬件设备直接控制
- 底层性能优化
- 硬件温度/电压监控
- CPU 亲和性绑定

---

## ⚡ 引擎层职责边界 (外置核心)

### ✅ 核心职责

#### 1. **挖矿算法实现**
- SHA256、Scrypt 等算法优化
- SIMD 指令集利用
- 硬件加速支持
- 算法特定优化

#### 2. **设备抽象和管理**
- 设备发现和识别
- 设备生命周期管理
- 设备状态监控
- 设备配置应用

#### 3. **硬件控制和监控**
- 频率、电压调节
- 温度、功耗监控
- 风扇速度控制
- 硬件错误检测

#### 4. **性能优化**
- CPU 亲和性绑定
- 内存优化
- 并发策略优化
- 平台特定优化

#### 5. **工作处理**
- 接收应用层分发的工作
- 工作分解和并行处理
- Nonce 搜索和验证
- 结果生成和上报

### ❌ **不负责的领域**
- 矿池通信
- 网络连接管理
- 全局配置管理
- Web 界面
- 系统级监控

---

## 🔌 接口定义和边界协议

### 标准化接口 (cgminer-core)

```rust
// 核心工厂接口
pub trait CoreFactory {
    async fn create_core(&self, config: CoreConfig) -> Result<Box<dyn MiningCore>>;
    fn validate_config(&self, config: &CoreConfig) -> Result<()>;
    fn default_config(&self) -> CoreConfig;
}

// 挖矿核心接口
pub trait MiningCore {
    async fn initialize(&mut self, config: CoreConfig) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn submit_work(&mut self, work: Work) -> Result<()>;
    async fn collect_results(&mut self) -> Result<Vec<MiningResult>>;
    async fn get_stats(&self) -> Result<CoreStats>;
}

// 设备接口
pub trait MiningDevice {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn submit_work(&mut self, work: Work) -> Result<()>;
    async fn get_result(&mut self) -> Result<Option<MiningResult>>;
    async fn get_stats(&self) -> Result<DeviceStats>;
}
```

### 数据流协议

```text
应用层 → 引擎层:
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Work      │───▶│ CoreConfig  │───▶│   Command   │
│   Config    │    │   Stats     │    │   Control   │
│   Control   │    │   Request   │    │             │
└─────────────┘    └─────────────┘    └─────────────┘

引擎层 → 应用层:
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ MiningResult│◄───│ DeviceStats │◄───│   Events    │
│ CoreStats   │    │   Metrics   │    │   Errors    │
│ Health      │    │   Status    │    │   Alerts    │
└─────────────┘    └─────────────┘    └─────────────┘
```

---

## 📊 依赖关系矩阵

| 模块类型 | cgminer-rs | cpu-btc-core | gpu-btc-core | asic-*-core | cgminer-core |
|----------|------------|--------------|--------------|-------------|--------------|
| **cgminer-rs** | - | ✅ 使用 | ✅ 使用 | ✅ 使用 | ✅ 实现 |
| **cpu-btc-core** | ❌ 禁止 | - | ❌ 禁止 | ❌ 禁止 | ✅ 实现 |
| **gpu-btc-core** | ❌ 禁止 | ❌ 禁止 | - | ❌ 禁止 | ✅ 实现 |
| **asic-*-core** | ❌ 禁止 | ❌ 禁止 | ❌ 禁止 | - | ✅ 实现 |
| **cgminer-core** | ❌ 禁止 | ❌ 禁止 | ❌ 禁止 | ❌ 禁止 | - |

**规则：**
- ✅ **允许**：上层可以依赖下层
- ❌ **禁止**：下层不能依赖上层，同层不能互相依赖

---

## 🚧 重构指导原则

### 1. **单向依赖原则**
- 应用层可以调用引擎层
- 引擎层不能直接调用应用层
- 通过回调、事件、接口实现反向通信

### 2. **接口稳定性原则**
- `cgminer-core` 接口保持向后兼容
- 版本化 API 变更
- 弃用警告和迁移指南

### 3. **职责单一原则**
- 每个模块只负责一个明确的领域
- 避免功能重叠和职责模糊
- 清晰的模块边界

### 4. **配置传递原则**
```text
全局配置 (cgminer-rs)
    ↓ 解析和验证
核心配置 (CoreConfig)
    ↓ 传递到引擎
设备配置 (DeviceConfig)
    ↓ 应用到硬件
硬件参数 (频率/电压/温度)
```

---

## 🛠️ 实施步骤

### 阶段 1: 清理重复导出 🧹
```bash
# 移除 cgminer-rs/src/lib.rs 中的重复导出
- pub use temperature::{TemperatureManager, TemperatureConfig};
- pub use performance::{PerformanceOptimizer, PerformanceConfig};
- pub use cpu_affinity::CpuAffinityManager;
- pub use device::SoftwareDevice;
```

### 阶段 2: 重构配置管理 ⚙️
```text
Before: cgminer-rs 配置 + cpu-btc-core 配置
After:  cgminer-rs 配置 → CoreConfig → 引擎层
```

### 阶段 3: 标准化接口 🔌
```text
确保所有外置核心都实现标准的 CoreFactory 和 MiningCore trait
移除直接的类型导出，只通过接口交互
```

### 阶段 4: 测试和验证 ✅
```text
集成测试验证边界清晰
性能测试确保无性能退化
兼容性测试确保现有功能正常
```

---

## 📋 边界检查清单

### ✅ 应用层检查
- [ ] 不包含具体挖矿算法
- [ ] 不直接控制硬件设备
- [ ] 不重复导出引擎层功能
- [ ] 专注于服务编排和用户界面

### ✅ 引擎层检查
- [ ] 不处理网络连接
- [ ] 不管理全局配置
- [ ] 不提供 Web 界面
- [ ] 专注于挖矿性能和硬件控制

### ✅ 接口检查
- [ ] 通过 cgminer-core 标准接口通信
- [ ] 没有直接的类型依赖
- [ ] 版本兼容性保证
- [ ] 清晰的错误处理

---

## 🎯 成功指标

### 技术指标
- **代码重复率** < 10%
- **模块耦合度** < 20%
- **接口稳定性** > 95%
- **构建时间** 减少 30%

### 维护指标
- **新核心集成时间** < 2 天
- **配置变更影响范围** 单模块
- **错误定位时间** < 30 分钟
- **功能测试覆盖率** > 90%

---

## 📚 相关文档

- [API 接口文档](./api-reference.md)
- [核心开发指南](./core-development-guide.md)
- [配置参考](./configuration-reference.md)
- [故障排除指南](./troubleshooting.md)

---

**最后更新**: 2024-12-19
**版本**: 1.0
**状态**: 待实施
