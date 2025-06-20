# CGMiner-RS SOCKS5代理支持

CGMiner-RS 现在完全支持通过SOCKS5代理连接到矿池，提供了最简洁的命令行使用方式。

## 🚀 最简洁的使用方式

**您提到的格式完全支持！** 直接在代理URL中包含认证信息是最便捷的方式：

```bash
# 您的示例格式 - 完美支持！
./cgminer-rs \
  --proxy socks5+tls://test22:cc112233d@54.248.128.73:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

## 🎯 支持的代理格式

### 1. SOCKS5+TLS（推荐，更安全）
```bash
--proxy socks5+tls://username:password@host:port
```

### 2. 标准SOCKS5
```bash
--proxy socks5://username:password@host:port
```

### 3. 无认证SOCKS5
```bash
--proxy socks5://host:port
```

## 💡 实际使用示例

### 使用您的代理服务器
```bash
# 直接使用您提供的代理格式
./cgminer-rs \
  --proxy socks5+tls://test22:cc112233d@54.248.128.73:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x
```

### 其他常见场景
```bash
# 本地Shadowsocks代理
./cgminer-rs \
  --proxy socks5://127.0.0.1:1080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x

# 企业代理环境
./cgminer-rs \
  --proxy socks5://domain\\user:password@corporate-proxy.com:1080 \
  --pool stratum+tcp://pool.example.com:4444 \
  --user worker1 \
  --pass password123
```

## 🔧 CLI选项说明

| 选项 | 说明 | 示例 |
|------|------|------|
| `--proxy` | SOCKS5代理URL（推荐在URL中包含认证） | `socks5+tls://user:pass@host:port` |
| `--proxy-user` | 代理用户名（可选，URL中有认证时不需要） | `username` |
| `--proxy-pass` | 代理密码（可选，URL中有认证时不需要） | `password` |
| `-o, --pool` | 矿池URL | `stratum+tcp://btc.f2pool.com:1314` |
| `-u, --user` | 矿池用户名 | `kayuii.bbt` |
| `-p, --pass` | 矿池密码 | `x` |

## ✅ 优势

1. **一行搞定**：认证信息直接包含在URL中，无需额外参数
2. **安全性**：支持SOCKS5+TLS加密传输
3. **灵活性**：支持多种认证方式
4. **兼容性**：与现有配置文件完全兼容
5. **便捷性**：CLI参数覆盖配置文件设置

## 🔍 认证优先级

当同时提供多种认证方式时：
1. CLI参数 (`--proxy-user`, `--proxy-pass`) 优先级最高
2. 代理URL中的认证信息 (`socks5://user:pass@host:port`) 次之

**建议**：直接在URL中包含认证信息，这样最简洁！

## 🛠️ 故障排除

### 查看详细日志
```bash
./cgminer-rs \
  --proxy socks5+tls://test22:cc112233d@54.248.128.73:8080 \
  --pool stratum+tcp://btc.f2pool.com:1314 \
  --user kayuii.bbt \
  --pass x \
  --log-level debug
```

### 成功连接的日志示例
```
🔧 CLI arguments applied to configuration
   🌐 Proxy: socks5+tls://test22:cc112233d@54.248.128.73:8080
   🏊 Pool: stratum+tcp://btc.f2pool.com:1314
   👤 User: kayuii.bbt

🔗 [Pool 0] 通过SOCKS5+TLS代理连接: 54.248.128.73:8080 -> btc.f2pool.com:1314
✅ SOCKS5+TLS代理连接建立成功: 54.248.128.73:8080 -> btc.f2pool.com:1314
```

## 📚 更多信息

- 详细文档：`examples/cli-socks5-usage.md`
- 配置文件示例：`examples/socks5-proxy-config.toml`
- 完整的SOCKS5文档：`docs/SOCKS5_PROXY.md`

---

**您的格式 `socks5+tls://test22:cc112233d@54.248.128.73:8080` 是完全支持的，这确实是最方便的使用方式！** 🎉
