# CGMiner-RS 架构设计文档

## 概述

CGMiner-RS 是对原有 C 语言版本 cgminer 的 Rust 重构版本，专门针对 Maijie L7 ASIC 矿机进行优化。本项目采用现代化的架构设计，提供更好的内存安全性、并发性能和可维护性。

## 核心设计原则

1. **内存安全**: 利用 Rust 的所有权系统防止内存泄漏和缓冲区溢出
2. **零成本抽象**: 保持与 C 版本相当的性能
3. **异步优先**: 使用 async/await 处理 I/O 密集型操作
4. **模块化设计**: 清晰的模块边界和接口定义
5. **可测试性**: 内置测试支持和模拟框架

## 系统架构

### 高层架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    CGMiner-RS Application                   │
├─────────────────────────────────────────────────────────────┤
│                      API Server                            │
│                   (HTTP/WebSocket)                         │
├─────────────────────────────────────────────────────────────┤
│                   Mining Manager                           │
│              (Coordination & Control)                      │
├─────────────┬─────────────┬─────────────┬─────────────────┤
│   Device    │    Work     │    Pool     │   Monitoring    │
│  Manager    │   Manager   │   Manager   │    System       │
├─────────────┼─────────────┼─────────────┼─────────────────┤
│ Chain Ctrl  │ Work Queue  │ Stratum     │   Metrics       │
│ Chain Ctrl  │ Result      │ Connection  │   Logging       │
│ Chain Ctrl  │ Queue       │ Pool        │   Alerting      │
├─────────────┴─────────────┴─────────────┴─────────────────┤
│                Hardware Abstraction Layer                  │
├─────────────────────────────────────────────────────────────┤
│              C FFI Interface (Hardware Drivers)            │
│                  (Maijie L7 Drivers)                      │
└─────────────────────────────────────────────────────────────┘
```

## 核心模块设计

### 1. Mining Manager (挖矿管理器)

**职责:**
- 协调各个子系统的工作
- 管理挖矿生命周期
- 处理系统级事件和错误

**接口:**
```rust
pub struct MiningManager {
    device_manager: Arc<DeviceManager>,
    work_manager: Arc<WorkManager>,
    pool_manager: Arc<PoolManager>,
    monitoring: Arc<MonitoringSystem>,
}

impl MiningManager {
    pub async fn start(&self) -> Result<(), MiningError>;
    pub async fn stop(&self) -> Result<(), MiningError>;
    pub async fn get_status(&self) -> SystemStatus;
}
```

### 2. Device Manager (设备管理器)

**职责:**
- 管理所有挖矿设备
- 设备初始化和配置
- 设备状态监控

**接口:**
```rust
pub struct DeviceManager {
    devices: Vec<Arc<MiningDevice>>,
    config: DeviceConfig,
}

impl DeviceManager {
    pub async fn initialize_devices(&mut self) -> Result<(), DeviceError>;
    pub async fn get_device_status(&self, device_id: u32) -> DeviceStatus;
    pub async fn restart_device(&self, device_id: u32) -> Result<(), DeviceError>;
}
```

### 3. Work Manager (工作管理器)

**职责:**
- 管理挖矿工作队列
- 分发工作到设备
- 收集挖矿结果

**接口:**
```rust
pub struct WorkManager {
    work_queue: Arc<Mutex<VecDeque<Work>>>,
    result_queue: Arc<Mutex<VecDeque<MiningResult>>>,
}

impl WorkManager {
    pub async fn submit_work(&self, work: Work) -> Result<(), WorkError>;
    pub async fn get_next_work(&self) -> Option<Work>;
    pub async fn submit_result(&self, result: MiningResult) -> Result<(), WorkError>;
}
```

### 4. Pool Manager (矿池管理器)

**职责:**
- 管理矿池连接
- 实现 Stratum 协议
- 处理矿池切换和故障转移

**接口:**
```rust
pub struct PoolManager {
    pools: Vec<Arc<Pool>>,
    active_pool: Arc<RwLock<Option<Arc<Pool>>>>,
    strategy: PoolStrategy,
}

impl PoolManager {
    pub async fn connect_to_pools(&self) -> Result<(), PoolError>;
    pub async fn submit_share(&self, share: Share) -> Result<(), PoolError>;
    pub async fn get_work(&self) -> Result<Work, PoolError>;
}
```

## 数据流设计

### 挖矿数据流

```
Pool → Work Manager → Device Manager → Hardware Driver
  ↑                                           ↓
  └── Pool Manager ← Work Manager ← Result ←──┘
```

### 监控数据流

```
Hardware → Device Manager → Monitoring System → API Server
                                ↓
                           Metrics Storage
```

## 错误处理策略

### 错误类型层次

```rust
#[derive(thiserror::Error, Debug)]
pub enum MiningError {
    #[error("Device error: {0}")]
    Device(#[from] DeviceError),
    
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
    
    #[error("Work error: {0}")]
    Work(#[from] WorkError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Hardware error: {0}")]
    Hardware(String),
}
```

### 错误恢复机制

1. **设备级错误**: 自动重启设备，记录错误日志
2. **矿池错误**: 自动切换到备用矿池
3. **系统级错误**: 优雅关闭，保存状态
4. **硬件错误**: 隔离故障设备，继续其他设备工作

## 并发模型

### 任务分配

- **主线程**: 系统协调和用户接口
- **设备线程池**: 每个设备一个专用任务
- **网络线程池**: 处理矿池通信
- **监控线程**: 定期收集系统状态

### 同步机制

- **Arc<Mutex<T>>**: 共享可变状态
- **Arc<RwLock<T>>**: 读多写少的共享状态
- **mpsc channels**: 异步消息传递
- **broadcast channels**: 事件通知

## 配置管理

### 配置文件结构

```toml
[general]
log_level = "info"
api_port = 4028

[devices]
auto_detect = true
chains = [
    { id = 0, frequency = 500, voltage = 850 },
    { id = 1, frequency = 500, voltage = 850 },
]

[pools]
strategy = "failover"

[[pools.pool]]
url = "stratum+tcp://pool1.example.com:4444"
user = "username"
password = "password"

[[pools.pool]]
url = "stratum+tcp://pool2.example.com:4444"
user = "username"
password = "password"
```

## 性能优化

### 关键优化点

1. **零拷贝数据传输**: 使用 `bytes` crate 避免不必要的内存拷贝
2. **批量处理**: 批量提交工作和结果
3. **连接池**: 复用网络连接
4. **内存池**: 预分配常用数据结构
5. **SIMD优化**: 在适当的地方使用向量化指令

### 性能监控

- **延迟监控**: 工作分发和结果收集的延迟
- **吞吐量监控**: 每秒处理的工作数量
- **资源使用**: CPU、内存、网络使用情况
- **设备效率**: 每个设备的算力和功耗

## 安全考虑

### 内存安全

- 使用 Rust 的所有权系统防止内存错误
- 避免使用 `unsafe` 代码，除非在 FFI 边界
- 定期进行内存泄漏检测

### 网络安全

- 使用 TLS 加密矿池连接
- 验证矿池证书
- 实现连接超时和重试机制

### 访问控制

- API 访问权限控制
- 配置文件权限保护
- 日志敏感信息脱敏

## 测试策略

### 单元测试

- 每个模块都有对应的单元测试
- 使用 `mockall` 进行依赖注入和模拟
- 测试覆盖率目标: 80%+

### 集成测试

- 端到端的挖矿流程测试
- 矿池连接和协议测试
- 设备故障恢复测试

### 性能测试

- 使用 `criterion` 进行基准测试
- 压力测试和长时间运行测试
- 内存使用和泄漏测试

## 部署和运维

### 构建和打包

- 支持交叉编译到 ARM 平台
- Docker 容器化部署
- 静态链接减少依赖

### 监控和日志

- 结构化日志输出
- Prometheus 指标导出
- 健康检查端点

### 升级和维护

- 热重载配置
- 优雅关闭机制
- 版本兼容性保证
