# CGMiner-RS 项目结构

本文档描述了 CGMiner-RS 项目的目录结构和文件组织方式，遵循 Rust 项目的最佳实践。

## 目录结构

```
cgminer_rs/
├── src/                    # 主要源代码
│   ├── main.rs            # 主程序入口
│   ├── lib.rs             # 库入口
│   ├── bin/               # 二进制程序
│   │   ├── f2pool_virtual.rs      # F2Pool 虚拟挖矿器
│   │   └── test_f2pool_config.rs  # F2Pool 配置测试
│   ├── api/               # API 相关代码
│   ├── config/            # 配置管理
│   ├── device/            # 设备驱动
│   ├── mining/            # 挖矿逻辑
│   ├── monitoring/        # 监控系统
│   └── pool/              # 矿池连接
├── examples/              # 示例代码和配置
│   ├── configs/           # 示例配置文件
│   │   ├── f2pool_simple.toml     # F2Pool 简化配置
│   │   └── f2pool_config.toml     # F2Pool 完整配置
│   ├── f2pool_virtual_miner.rs    # F2Pool 虚拟挖矿示例
│   ├── run_virtual_miner.rs       # 虚拟挖矿器示例
│   └── virtual_miner.rs           # 基础虚拟挖矿示例
├── scripts/               # 构建和运行脚本
│   ├── run_f2pool.sh      # F2Pool 挖矿启动脚本
│   ├── run_virtual.sh     # 虚拟挖矿启动脚本
│   └── verify_build.sh    # 构建验证脚本
├── tests/                 # 集成测试
├── benches/               # 性能测试
├── docs/                  # 文档
└── target/                # 编译输出 (自动生成)
```

## 文件说明

### 核心二进制程序 (`src/bin/`)

- **f2pool_virtual.rs**: F2Pool 虚拟挖矿器，使用真实的 F2Pool 配置进行虚拟挖矿
- **test_f2pool_config.rs**: F2Pool 配置测试工具，验证矿池配置的正确性

### 示例程序 (`examples/`)

- **f2pool_virtual_miner.rs**: 完整的 F2Pool 虚拟挖矿示例
- **run_virtual_miner.rs**: 基础虚拟挖矿器示例
- **virtual_miner.rs**: 简单的虚拟挖矿演示

### 配置文件 (`examples/configs/`)

- **f2pool_simple.toml**: F2Pool 简化配置，包含基本的矿池和设备设置
- **f2pool_config.toml**: F2Pool 完整配置，包含所有可配置选项

### 脚本 (`scripts/`)

- **run_f2pool.sh**: 一键启动 F2Pool 虚拟挖矿器
- **run_virtual.sh**: 启动基础虚拟挖矿器
- **verify_build.sh**: 验证项目构建

## 使用方法

### 运行 F2Pool 虚拟挖矿器

```bash
# 方法 1: 使用脚本
./scripts/run_f2pool.sh

# 方法 2: 直接使用 cargo
cargo run --bin f2pool_virtual

# 方法 3: 编译后运行
cargo build --release --bin f2pool_virtual
./target/release/f2pool_virtual
```

### 测试 F2Pool 配置

```bash
cargo run --bin test_f2pool_config
```

### 运行示例程序

```bash
# 运行 F2Pool 虚拟挖矿示例
cargo run --example f2pool_virtual_miner

# 运行基础虚拟挖矿示例
cargo run --example virtual_miner
```

## 开发规范

1. **二进制程序**: 放在 `src/bin/` 目录下，文件名使用下划线分隔
2. **示例代码**: 放在 `examples/` 目录下，可以通过 `cargo run --example` 运行
3. **配置文件**: 示例配置放在 `examples/configs/` 目录下
4. **脚本文件**: 放在 `scripts/` 目录下，使用 `.sh` 扩展名
5. **文档**: 放在 `docs/` 目录下，使用 Markdown 格式

## 构建和测试

```bash
# 构建所有二进制程序
cargo build --release

# 运行测试
cargo test

# 运行基准测试
cargo bench

# 检查代码格式
cargo fmt --check

# 代码检查
cargo clippy
```

这种结构遵循了 Rust 社区的最佳实践，使项目更加规范和易于维护。
