# CGMiner-RS 简单演示

## 概述

这是一个简化的 CGMiner-RS 演示程序，模拟 cgminer 风格的输出，展示了新架构的核心功能。

## 特性

- ✅ 模拟 cgminer 风格的时间戳日志输出
- ✅ 支持本地矿池转发连接 (127.0.0.1:1314)
- ✅ 展示算力计算重构成果
- ✅ 测试 meets_target 函数
- ✅ 无复杂日志系统，输出简洁易读

## 运行方法

### 方法1：直接运行
```bash
cd /Users/gecko/project/linux/cgminer_rs
cargo run --example multi_device_demo --features=cpu-btc
```

### 方法2：使用脚本
```bash
./run_demo.sh
```

## 输出示例

```
[16:42:29] Started cgminer-rs 1.0.0
[16:42:29] Loading configuration...
[16:42:29] Using default configuration
[16:42:29] Pool 0: 127.0.0.1:1314
[16:42:29] Initializing mining cores...
[16:42:29] Found 1 mining core(s)
[16:42:29] Core: Software Mining Core v0.2.0 (software)
[16:42:29] Creating CPU mining core...
[16:42:29] Core cpu-btc_xxx created successfully
[16:42:29] Core cpu-btc_xxx started
[16:42:30] Devices: 4 | Hashrate: 10000000000.00 H/s
[16:42:30] Connecting to pool 127.0.0.1:1314...
[16:42:31] Pool 0: Connected to 127.0.0.1:1314
[16:42:31] Pool 0: Authorized worker
[16:42:31] Pool 0: New block detected
[16:42:31] Work received from pool 0
[16:42:31] Mining started...
[16:42:36] (5s): 0.00H/s | A:0 R:0 HW:0 WU:0.0/m
[16:42:39] Accepted cpu-btc_xxx Diff 1/1 127.0.0.1:1314 58ms
[16:42:41] (10s): 0.00H/s | A:0 R:0 HW:0 WU:0.0/m
[16:42:47] Accepted cpu-btc_xxx Diff 1/1 127.0.0.1:1314 66ms
[16:43:01] Testing target validation...
[16:43:01] Target test: Easy=true Hard=false
[16:43:01] Shutting down...
[16:43:01] Core cpu-btc_xxx stopped
[16:43:01] Core cpu-btc_xxx removed
[16:43:01] cgminer-rs shutdown complete
```

## 矿池连接

演示程序模拟连接到本地矿池转发：
- **地址**: 127.0.0.1:1314
- **协议**: 支持标准 Stratum 协议
- **认证**: 自动授权工作节点

如果您有本地矿池转发运行在该端口，演示程序会模拟真实的矿池连接过程。

## 架构重构成果展示

### 1. 算力计算分层
- **设备层**: 记录原始哈希数据
- **核心层**: 聚合设备数据计算核心算力
- **应用层**: 汇总核心算力显示总算力

### 2. meets_target 函数统一
- 所有核心使用 `cgminer-core::meets_target()` 统一验证逻辑
- 避免重复实现，提高一致性
- 演示中测试了容易目标和困难目标的验证

### 3. 清晰的职责边界
- **应用层**: 配置管理、矿池连接、统计展示
- **引擎层**: 挖矿算法、设备控制、性能优化
- **接口层**: 标准化的通信协议

## 代码结构

```
multi_device_demo.rs
├── cgminer_timestamp()     # 时间戳格式化
├── cgminer_log!()          # 日志输出宏
├── main()                  # 主程序逻辑
│   ├── 配置管理
│   ├── 核心注册表初始化
│   ├── 核心创建和启动
│   ├── 矿池连接模拟
│   ├── 挖矿运行和统计
│   ├── meets_target 测试
│   └── 优雅关闭
└── format_hashrate()       # 算力格式化
```

## 技术要点

- **无依赖日志系统**: 使用简单的 `println!` 宏
- **cgminer 风格输出**: HH:MM:SS 时间戳格式
- **算力单位自动转换**: H/s, KH/s, MH/s, GH/s, TH/s
- **模拟真实挖矿**: 包括接受 share、工作单元等
- **错误处理**: 优雅的错误处理和资源清理

## 故障排除

### 编译错误
确保启用 CPU-BTC 功能：
```bash
cargo run --example multi_device_demo --features=cpu-btc
```

### 运行时错误
- 检查 Cargo.toml 依赖配置
- 确保 cgminer-cpu-btc-core 正确编译
- 查看详细错误信息进行调试

## 扩展功能

要添加真实的矿池连接，可以：
1. 实现 Stratum 协议客户端
2. 添加真实的工作分发逻辑
3. 集成真实的哈希验证
4. 添加矿池故障转移机制

---

**最后更新**: 2024-12-19
**版本**: 1.0
**状态**: 可运行
