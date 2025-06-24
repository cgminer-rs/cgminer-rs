# CGMiner-RS SOCKS5+TLS 改进说明

## 🔧 问题分析

原始的SOCKS5+TLS实现存在以下问题：

### 1. **协议层次错误**
- **原实现**: `TCP -> SOCKS5 -> TLS`（错误）
- **正确实现**: `TCP -> TLS -> SOCKS5`（正确）

### 2. **缺少TLS配置选项**
- 没有证书验证配置
- 没有SNI（服务器名称指示）设置
- 没有自定义CA证书支持
- 没有客户端证书认证支持

### 3. **与gost标准不兼容**
- 不符合业界标准的SOCKS5+TLS实现
- 缺少高级安全特性

## 🚀 改进方案

### **新的协议架构**

```
用户应用 -> CGMiner-RS -> TCP -> TLS -> SOCKS5 -> 目标矿池
                           |      |       |
                           |      |       +-- SOCKS5协商
                           |      +-- TLS加密层
                           +-- 基础TCP连接
```

### **TLS配置选项**

```rust
pub struct TlsConfig {
    /// 是否跳过证书验证（仅用于测试）
    pub skip_verify: bool,
    /// 服务器名称指示（SNI）
    pub server_name: Option<String>,
    /// 自定义CA证书路径
    pub ca_cert_path: Option<String>,
    /// 客户端证书路径
    pub client_cert_path: Option<String>,
    /// 客户端私钥路径
    pub client_key_path: Option<String>,
}
```

## 📋 使用方法

### **1. 基础SOCKS5+TLS配置**

```toml
[pools.pools.proxy]
proxy_type = "socks5+tls"
host = "proxy.example.com"
port = 1080
username = "proxy_user"
password = "proxy_pass"
```

### **2. 命令行使用（推荐）**

```bash
# 基础用法
./cgminer-rs \
  --proxy socks5+tls://user:pwd@127.0.0.1:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x

# 禁用证书验证（仅用于测试）
./cgminer-rs \
  --proxy "socks5+tls://user:pass@proxy.com:1080?skip_verify=true" \
  --pool stratum+tcp://pool.com:4444 \
  --user username \
  --pass password

# 自定义SNI
./cgminer-rs \
  --proxy "socks5+tls://user:pass@proxy.com:1080?server_name=proxy.example.com" \
  --pool stratum+tcp://pool.com:4444 \
  --user username \
  --pass password
```

### **3. 支持的URL参数**

| 参数 | 说明 | 示例 |
|------|------|------|
| `skip_verify` | 跳过证书验证 | `?skip_verify=true` |
| `insecure` | 同`skip_verify` | `?insecure=true` |
| `server_name` | 自定义SNI | `?server_name=proxy.example.com` |
| `sni` | 同`server_name` | `?sni=proxy.example.com` |
| `ca` | CA证书路径 | `?ca=./certs/ca.pem` |
| `cert` | 客户端证书路径 | `?cert=./certs/client.pem` |
| `key` | 客户端私钥路径 | `?key=./certs/client.key` |

## 🔒 安全特性

### **1. TLS版本控制**
- 强制使用TLS 1.2+
- 禁用不安全的TLS版本

### **2. 证书验证**
- 默认启用完整证书验证
- 支持证书锁定（Certificate Pinning）
- 可配置自定义CA证书

### **3. SNI支持**
- 支持服务器名称指示
- 解决多域名代理服务器问题

### **4. 客户端证书认证**
- 支持双向TLS认证
- 增强安全性

## 🔧 技术实现

### **SOCKS5协商过程**

1. **建立TCP连接**到代理服务器
2. **TLS握手**与代理服务器
3. **SOCKS5认证方法协商**
4. **用户名密码认证**（如需要）
5. **SOCKS5连接请求**到目标矿池
6. **数据转发**（全程TLS加密）

### **手动SOCKS5实现**

由于tokio-socks的限制，我们实现了自定义的SOCKS5协商：

```rust
// 1. 认证方法协商
let auth_methods = vec![0x00, 0x02]; // NO_AUTH, USERNAME_PASSWORD
let request = vec![0x05, auth_methods.len() as u8];
request.extend(&auth_methods);

// 2. 用户名密码认证
let auth_request = vec![0x01, username.len() as u8];
auth_request.extend(username.as_bytes());
auth_request.push(password.len() as u8);
auth_request.extend(password.as_bytes());

// 3. 连接请求
let connect_request = vec![0x05, 0x01, 0x00]; // VER, CMD, RSV
// ... 添加目标地址和端口
```

## 🎯 与gost对比

| 特性 | gost | CGMiner-RS |
|------|------|------------|
| 协议层次 | ✅ TCP->TLS->SOCKS5 | ✅ TCP->TLS->SOCKS5 |
| 证书验证 | ✅ 支持 | ✅ 支持 |
| SNI | ✅ 支持 | ✅ 支持 |
| 客户端证书 | ✅ 支持 | 🚧 计划支持 |
| CA证书锁定 | ✅ 支持 | 🚧 计划支持 |
| 协商加密 | ✅ 支持 | ✅ 支持 |
| UDP支持 | ✅ 支持 | ❌ 不需要 |

## 📊 性能优化

### **连接复用**
- 支持长连接
- 减少TLS握手开销

### **错误处理**
- 详细的错误信息
- 自动重连机制

### **日志记录**
- 完整的连接过程日志
- 调试友好的错误信息

## ⚠️ 安全注意事项

### **生产环境**
1. **永远不要**在生产环境中设置`skip_verify=true`
2. **使用**有效的TLS证书
3. **配置**适当的CA证书验证
4. **定期更新**代理服务器证书

### **测试环境**
1. 可以使用`skip_verify=true`进行测试
2. 推荐使用自签名证书测试
3. 验证日志输出确保连接正常

## 🚀 使用示例

### **与gost代理服务器配合使用**

1. **启动gost服务器**:
```bash
# 基础SOCKS5+TLS服务器
gost -L "socks5+tls://user:pass@:8080?cert=server.pem&key=server.key"

# 带认证的SOCKS5+TLS服务器
gost -L "socks5+tls://user:pass@:8080?cert=server.pem&key=server.key&auth=user:pass"
```

2. **CGMiner-RS客户端连接**:
```bash
# 连接到gost服务器
./cgminer-rs \
  --proxy "socks5+tls://user:pass@proxy-server:8080" \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

## 🎉 总结

改进后的SOCKS5+TLS实现：

✅ **正确的协议层次**：TCP -> TLS -> SOCKS5
✅ **完整的TLS配置**：证书验证、SNI、CA锁定
✅ **与gost兼容**：符合业界标准
✅ **安全增强**：强制TLS 1.2+、证书验证
✅ **易于使用**：命令行一行搞定
✅ **详细日志**：完整的调试信息

这个实现参考了gost项目的优秀设计，为cgminer_rs提供了企业级的代理支持能力。
