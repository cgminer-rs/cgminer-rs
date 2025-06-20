# SOCKS5代理支持

CGMiner-RS 支持通过SOCKS5代理连接到矿池，这对于需要通过代理服务器访问矿池的环境非常有用。

## 支持的代理类型

1. **socks5://** - 标准SOCKS5代理
2. **socks5+tls://** - 带TLS加密的SOCKS5代理

## 配置方式

### 1. 在配置文件中配置代理

在 `cgminer.toml` 配置文件中为每个矿池配置代理：

```toml
[[pools.pools]]
name = "f2pool-via-proxy"
url = "stratum+tcp://btc.f2pool.com:1314"
username = "kayuii.bbt"
password = "x"
priority = 1
enabled = true

# SOCKS5代理配置
[pools.pools.proxy]
proxy_type = "socks5"           # 代理类型: socks5 或 socks5+tls
host = "127.0.0.1"              # 代理服务器地址
port = 1080                     # 代理服务器端口
username = "proxy_user"         # 代理认证用户名（可选）
password = "proxy_pass"         # 代理认证密码（可选）
```

### 2. 代理配置参数说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `proxy_type` | String | 是 | 代理类型，支持 "socks5" 和 "socks5+tls" |
| `host` | String | 是 | 代理服务器的IP地址或域名 |
| `port` | u16 | 是 | 代理服务器端口 |
| `username` | String | 否 | SOCKS5认证用户名 |
| `password` | String | 否 | SOCKS5认证密码 |

## 使用示例

### 示例1: 无认证的SOCKS5代理

```toml
[[pools.pools]]
name = "pool-via-socks5"
url = "stratum+tcp://pool.example.com:4444"
username = "your_username"
password = "your_password"
priority = 1
enabled = true

[pools.pools.proxy]
proxy_type = "socks5"
host = "127.0.0.1"
port = 1080
```

### 示例2: 带认证的SOCKS5代理

```toml
[[pools.pools]]
name = "pool-via-auth-socks5"
url = "stratum+tcp://pool.example.com:4444"
username = "your_username"
password = "your_password"
priority = 1
enabled = true

[pools.pools.proxy]
proxy_type = "socks5"
host = "proxy.example.com"
port = 1080
username = "proxy_user"
password = "proxy_pass"
```

### 示例3: SOCKS5+TLS代理

```toml
[[pools.pools]]
name = "pool-via-socks5-tls"
url = "stratum+tcp://secure.pool.com:4444"
username = "your_username"
password = "your_password"
priority = 1
enabled = true

[pools.pools.proxy]
proxy_type = "socks5+tls"
host = "secure-proxy.example.com"
port = 1080
username = "proxy_user"
password = "proxy_pass"
```

### 示例4: 直接连接（无代理）

```toml
[[pools.pools]]
name = "direct-pool"
url = "stratum+tcp://direct.pool.com:4444"
username = "your_username"
password = "your_password"
priority = 1
enabled = true
# 注意：没有 [pools.pools.proxy] 配置表示直接连接
```

## 常见代理服务器

### 本地SOCKS5代理

如果您在本地运行SOCKS5代理（如Shadowsocks、V2Ray等）：

```toml
[pools.pools.proxy]
proxy_type = "socks5"
host = "127.0.0.1"
port = 1080  # 根据您的代理软件配置调整端口
```

### 企业代理

如果您在企业环境中使用代理服务器：

```toml
[pools.pools.proxy]
proxy_type = "socks5"
host = "proxy.company.com"
port = 1080
username = "your_domain_user"
password = "your_domain_password"
```

## 故障排除

### 连接失败

如果代理连接失败，请检查：

1. **代理服务器状态**: 确保代理服务器正在运行且可访问
2. **网络连接**: 确保能够连接到代理服务器的IP和端口
3. **认证信息**: 如果使用认证，确保用户名和密码正确
4. **防火墙设置**: 确保防火墙允许连接到代理服务器

### 日志调试

启用调试日志来查看详细的连接信息：

```bash
cargo run -- --log-level debug
```

查看日志中的代理连接信息：

```
🔗 [Pool 0] 通过SOCKS5代理连接: 127.0.0.1:1080 -> btc.f2pool.com:1314
✅ SOCKS5代理连接建立成功: 127.0.0.1:1080 -> btc.f2pool.com:1314
```

### 常见错误

1. **连接超时**: 检查代理服务器地址和端口
2. **认证失败**: 检查用户名和密码
3. **协议错误**: 确保代理服务器支持SOCKS5协议
4. **TLS握手失败**: 对于socks5+tls，确保目标服务器支持TLS

## 性能考虑

- 使用代理会增加一定的延迟，这可能影响挖矿效率
- SOCKS5+TLS会增加额外的加密开销
- 建议选择地理位置较近的代理服务器以减少延迟

## 安全注意事项

- 确保代理服务器是可信的
- 对于敏感环境，建议使用socks5+tls以增加安全性
- 定期更换代理认证密码
- 避免在不安全的网络环境中使用无认证的代理
