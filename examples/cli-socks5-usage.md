# CGMiner-RS CLI SOCKS5代理使用指南

本文档展示如何通过命令行参数直接使用SOCKS5代理连接到矿池，无需修改配置文件。

## 🚀 快速开始

最简洁的使用方式是将认证信息直接包含在代理URL中：

```bash
# 一行命令搞定SOCKS5+TLS代理连接
./cgminer-rs \
  --proxy socks5+tls://user:pwd@127.0.0.1:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x

# 普通SOCKS5代理
./cgminer-rs \
  --proxy socks5://username:password@proxy.example.com:1080 \
  --pool stratum+tcp://pool.example.com:4444 \
  --user worker1 \
  --pass password123
```

这种方式无需额外的 `--proxy-user` 和 `--proxy-pass` 参数，更加简洁高效！

## 新增的CLI选项

### 代理相关选项

- `--proxy <URL>` - SOCKS5代理URL
- `--proxy-user <USERNAME>` - SOCKS5代理认证用户名
- `--proxy-pass <PASSWORD>` - SOCKS5代理认证密码

### 矿池相关选项

- `-o, --pool <URL>` - 矿池URL（覆盖配置文件）
- `-u, --user <USERNAME>` - 矿池用户名/工人名（覆盖配置文件）
- `-p, --pass <PASSWORD>` - 矿池密码（覆盖配置文件）

## 使用示例

### 1. 使用无认证SOCKS5代理

```bash
# 通过本地SOCKS5代理连接F2Pool
./cgminer-rs \
  --proxy socks5://127.0.0.1:1080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

### 2. 使用带认证的SOCKS5代理

```bash
# 通过带认证的SOCKS5代理连接矿池
./cgminer-rs \
  --proxy socks5://proxy.example.com:1080 \
  --proxy-user myuser \
  --proxy-pass mypass \
  --pool stratum+tcp://pool.example.com:4444 \
  --user worker1 \
  --pass password123
```

### 3. 使用SOCKS5+TLS代理

```bash
# 通过SOCKS5+TLS代理连接矿池（更安全）
./cgminer-rs \
  --proxy socks5+tls://secure-proxy.example.com:1080 \
  --proxy-user secureuser \
  --proxy-pass securepass \
  --pool stratum+tcp://secure.pool.com:4444 \
  --user worker1 \
  --pass password123
```

### 4. 在代理URL中包含认证信息（推荐方式）

```bash
# 认证信息可以直接包含在代理URL中，这是最简洁的方式
./cgminer-rs \
  --proxy socks5://username:password@proxy.example.com:1080 \
  --pool stratum+tcp://pool.example.com:4444 \
  --user worker1 \
  --pass password123

# 实际使用示例
./cgminer-rs \
  --proxy socks5+tls://user:pwd@127.0.0.1:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

### 5. 只覆盖代理设置（使用配置文件中的矿池）

```bash
# 只设置代理，矿池信息使用配置文件中的设置
./cgminer-rs --proxy socks5://127.0.0.1:1080
```

### 6. 只覆盖矿池设置（不使用代理）

```bash
# 只设置矿池，不使用代理
./cgminer-rs \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

### 7. 完整的命令行示例

```bash
# 完整的命令行参数示例
./cgminer-rs \
  --config /path/to/cgminer.toml \
  --proxy socks5://127.0.0.1:1080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x \
  --api-port 4028 \
  --log-level debug
```

## 参数优先级

CLI参数会覆盖配置文件中的相应设置：

1. **代理设置**: `--proxy` 参数会应用到所有启用的矿池
2. **矿池URL**: `--pool` 参数会覆盖第一个矿池的URL
3. **用户名**: `--user` 参数会覆盖第一个矿池的用户名
4. **密码**: `--pass` 参数会覆盖第一个矿池的密码

## 代理认证优先级

当同时提供多种认证方式时，优先级如下：

1. CLI参数 (`--proxy-user`, `--proxy-pass`)
2. 代理URL中的认证信息 (`socks5://user:pass@host:port`) **推荐使用**

**建议**：直接在代理URL中包含认证信息，这样更简洁，无需额外的认证参数。

## 错误处理

### 无效的代理URL

```bash
# 错误示例 - 不支持的协议
./cgminer-rs --proxy http://proxy.example.com:8080
# 错误: Unsupported proxy scheme: http. Use 'socks5' or 'socks5+tls'
```

### 缺少端口号

```bash
# 错误示例 - 缺少端口号
./cgminer-rs --proxy socks5://127.0.0.1
# 错误: Proxy URL must include a port
```

### 无效的主机地址

```bash
# 错误示例 - 无效的主机
./cgminer-rs --proxy socks5://:1080
# 错误: Proxy URL must include a host
```

## 调试和故障排除

### 启用调试日志

```bash
# 启用调试日志查看详细的代理连接信息
./cgminer-rs \
  --proxy socks5://127.0.0.1:1080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x \
  --log-level debug
```

### 查看应用的配置

启动时会显示CLI参数覆盖的信息：

```
🔧 CLI arguments applied to configuration
   🌐 Proxy: socks5://127.0.0.1:1080
   🏊 Pool: stratum+tcp://btc.f2pool.com:1314
   👤 User: kayuii.bbt
```

### 代理连接日志

成功的代理连接会显示如下日志：

```
🔗 [Pool 0] 通过SOCKS5代理连接: 127.0.0.1:1080 -> btc.f2pool.com:1314
✅ SOCKS5代理连接建立成功: 127.0.0.1:1080 -> btc.f2pool.com:1314
```

## 常见使用场景

### 1. 开发和测试

```bash
# 快速测试不同的代理和矿池组合
./cgminer-rs --proxy socks5://127.0.0.1:1080 --pool stratum+tcp://testpool.com:4444 --user test --pass test
```

### 2. 临时代理切换

```bash
# 临时使用不同的代理，不修改配置文件
./cgminer-rs --proxy socks5://backup-proxy.example.com:1080
```

### 3. 企业环境部署

```bash
# 在企业环境中使用公司代理
./cgminer-rs \
  --proxy socks5://corporate-proxy.company.com:1080 \
  --proxy-user domain\\username \
  --proxy-pass password
```

### 4. 自动化脚本

```bash
#!/bin/bash
# 自动化部署脚本
PROXY_HOST="127.0.0.1"
PROXY_PORT="1080"
POOL_URL="stratum+tcp://btc.f2pool.com:1314"
WORKER_NAME="kayuii.bbt"

./cgminer-rs \
  --proxy "socks5://${PROXY_HOST}:${PROXY_PORT}" \
  --pool "${POOL_URL}" \
  --user "${WORKER_NAME}" \
  --pass "x"
```

## 注意事项

1. **安全性**: 避免在命令行中直接输入敏感密码，考虑使用环境变量
2. **性能**: 代理连接会增加延迟，选择地理位置较近的代理服务器
3. **稳定性**: 确保代理服务器稳定可靠，避免频繁断线
4. **兼容性**: 确保代理服务器支持SOCKS5协议

## 环境变量支持

为了提高安全性，可以使用环境变量：

```bash
# 设置环境变量
export CGMINER_PROXY="socks5://127.0.0.1:1080"
export CGMINER_PROXY_USER="username"
export CGMINER_PROXY_PASS="password"
export CGMINER_POOL="stratum+tcp://btc.f2pool.com:1314"
export CGMINER_USER="kayuii.bbt"
export CGMINER_PASS="x"

# 在脚本中使用
./cgminer-rs \
  --proxy "${CGMINER_PROXY}" \
  --proxy-user "${CGMINER_PROXY_USER}" \
  --proxy-pass "${CGMINER_PROXY_PASS}" \
  --pool "${CGMINER_POOL}" \
  --user "${CGMINER_USER}" \
  --pass "${CGMINER_PASS}"
```
