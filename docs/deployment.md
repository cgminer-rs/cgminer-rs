# CGMiner-RS 部署指南

本文档详细介绍如何在生产环境中部署 CGMiner-RS。

## 系统要求

### 硬件要求

- **CPU**: ARM64 或 x86_64 架构
- **内存**: 最少 512MB RAM，推荐 1GB+
- **存储**: 最少 100MB 可用空间
- **网络**: 稳定的互联网连接

### 软件要求

- **操作系统**: Linux (Ubuntu 18.04+, CentOS 7+, Debian 9+)
- **内核版本**: 4.4+
- **权限**: root 或具有硬件访问权限的用户

### 支持的硬件

- Maijie L7 ASIC 矿机
- S19 系列矿机
- 其他兼容的 ASIC 设备

## 安装方式

### 方式一：预编译二进制文件

1. **下载最新版本**

```bash
# 下载适合您架构的二进制文件
wget https://github.com/your-org/cgminer-rs/releases/latest/download/cgminer-rs-linux-aarch64.tar.gz

# 解压
tar -xzf cgminer-rs-linux-aarch64.tar.gz
cd cgminer-rs
```

2. **安装到系统**

```bash
# 复制二进制文件
sudo cp cgminer-rs /usr/local/bin/
sudo chmod +x /usr/local/bin/cgminer-rs

# 创建配置目录
sudo mkdir -p /etc/cgminer-rs
sudo cp cgminer.toml /etc/cgminer-rs/

# 创建日志目录
sudo mkdir -p /var/log/cgminer-rs
sudo chown $USER:$USER /var/log/cgminer-rs
```

### 方式二：从源码编译

1. **安装 Rust 工具链**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. **安装系统依赖**

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev git

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel git
```

3. **编译项目**

```bash
git clone https://github.com/your-org/cgminer-rs.git
cd cgminer-rs
cargo build --release

# 安装
sudo cp target/release/cgminer-rs /usr/local/bin/
```

### 方式三：Docker 部署

1. **使用预构建镜像**

```bash
docker pull cgminer-rs:latest
```

2. **运行容器**

```bash
docker run -d \
  --name cgminer-rs \
  --privileged \
  --restart unless-stopped \
  -v /dev:/dev \
  -v /etc/cgminer-rs:/app/config \
  -v /var/log/cgminer-rs:/app/logs \
  -p 4028:4028 \
  -p 9090:9090 \
  cgminer-rs:latest
```

## 配置

### 基础配置

创建配置文件 `/etc/cgminer-rs/cgminer.toml`:

```toml
[general]
log_level = "info"
log_file = "/var/log/cgminer-rs/cgminer.log"
pid_file = "/var/run/cgminer-rs.pid"

[devices]
auto_detect = true
scan_interval = 5

[[devices.chains]]
id = 0
enabled = true
frequency = 500
voltage = 850
auto_tune = true
chip_count = 76

[[devices.chains]]
id = 1
enabled = true
frequency = 500
voltage = 850
auto_tune = true
chip_count = 76

[pools]
strategy = "failover"
failover_timeout = 30
retry_interval = 10

[[pools.pools]]
url = "stratum+tcp://your-pool.com:4444"
user = "your-username"
password = "your-password"
priority = 1
enabled = true

[[pools.pools]]
url = "stratum+tcp://backup-pool.com:4444"
user = "your-username"
password = "your-password"
priority = 2
enabled = true

[api]
enabled = true
bind_address = "0.0.0.0"
port = 4028
auth_token = "your-secret-token"

[monitoring]
enabled = true
metrics_interval = 30
prometheus_port = 9090

[monitoring.alert_thresholds]
temperature_warning = 80.0
temperature_critical = 90.0
hashrate_drop_percent = 20.0
error_rate_percent = 5.0
```

### 高级配置

#### 自动调优配置

```toml
[devices.auto_tuning]
enabled = true
target_efficiency = 0.065  # MH/J
max_temperature = 85.0
tuning_interval = 3600     # 1 hour
```

#### 安全配置

```toml
[security]
enable_tls = true
cert_file = "/etc/cgminer-rs/server.crt"
key_file = "/etc/cgminer-rs/server.key"
allowed_ips = ["192.168.1.0/24", "10.0.0.0/8"]
```

#### 日志配置

```toml
[logging]
level = "info"
format = "json"
rotation = "daily"
max_files = 7
max_size = "100MB"

[logging.targets]
console = true
file = true
syslog = false
```

## 系统服务配置

### Systemd 服务

创建服务文件 `/etc/systemd/system/cgminer-rs.service`:

```ini
[Unit]
Description=CGMiner-RS Bitcoin Miner
After=network.target
Wants=network.target

[Service]
Type=simple
User=cgminer
Group=cgminer
ExecStart=/usr/local/bin/cgminer-rs --config /etc/cgminer-rs/cgminer.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=cgminer-rs

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/cgminer-rs /var/run

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### 创建专用用户

```bash
# 创建系统用户
sudo useradd -r -s /bin/false cgminer

# 设置权限
sudo chown -R cgminer:cgminer /etc/cgminer-rs
sudo chown -R cgminer:cgminer /var/log/cgminer-rs

# 添加硬件访问权限
sudo usermod -a -G dialout cgminer
```

### 启动服务

```bash
# 重新加载 systemd
sudo systemctl daemon-reload

# 启用服务
sudo systemctl enable cgminer-rs

# 启动服务
sudo systemctl start cgminer-rs

# 检查状态
sudo systemctl status cgminer-rs
```

## 网络配置

### 防火墙设置

```bash
# UFW (Ubuntu)
sudo ufw allow 4028/tcp comment "CGMiner-RS API"
sudo ufw allow 9090/tcp comment "Prometheus metrics"

# iptables
sudo iptables -A INPUT -p tcp --dport 4028 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 9090 -j ACCEPT
```

### 反向代理 (Nginx)

```nginx
server {
    listen 80;
    server_name miner.example.com;
    
    location /api/ {
        proxy_pass http://localhost:4028/api/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location /ws {
        proxy_pass http://localhost:4028/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
    
    location /metrics {
        proxy_pass http://localhost:9090/metrics;
        allow 192.168.1.0/24;
        deny all;
    }
}
```

## 监控配置

### Prometheus 配置

```yaml
# prometheus.yml
global:
  scrape_interval: 30s

scrape_configs:
  - job_name: 'cgminer-rs'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 30s
    metrics_path: /metrics
```

### Grafana 仪表板

导入预配置的 Grafana 仪表板：

```bash
# 下载仪表板配置
wget https://raw.githubusercontent.com/your-org/cgminer-rs/main/monitoring/grafana/dashboard.json

# 导入到 Grafana
curl -X POST \
  http://admin:admin@localhost:3000/api/dashboards/db \
  -H 'Content-Type: application/json' \
  -d @dashboard.json
```

## 备份和恢复

### 备份配置

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/cgminer-rs/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# 备份配置文件
cp -r /etc/cgminer-rs "$BACKUP_DIR/"

# 备份日志
cp -r /var/log/cgminer-rs "$BACKUP_DIR/"

# 创建压缩包
tar -czf "$BACKUP_DIR.tar.gz" -C /backup/cgminer-rs "$(basename $BACKUP_DIR)"
rm -rf "$BACKUP_DIR"

echo "Backup created: $BACKUP_DIR.tar.gz"
```

### 恢复配置

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE="$1"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.tar.gz>"
    exit 1
fi

# 停止服务
sudo systemctl stop cgminer-rs

# 解压备份
tar -xzf "$BACKUP_FILE" -C /tmp/

# 恢复配置
sudo cp -r /tmp/*/cgminer-rs /etc/

# 设置权限
sudo chown -R cgminer:cgminer /etc/cgminer-rs

# 启动服务
sudo systemctl start cgminer-rs

echo "Configuration restored from $BACKUP_FILE"
```

## 性能调优

### 系统优化

```bash
# 增加文件描述符限制
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# 优化网络参数
echo "net.core.rmem_max = 16777216" >> /etc/sysctl.conf
echo "net.core.wmem_max = 16777216" >> /etc/sysctl.conf
echo "net.ipv4.tcp_rmem = 4096 87380 16777216" >> /etc/sysctl.conf
echo "net.ipv4.tcp_wmem = 4096 65536 16777216" >> /etc/sysctl.conf

# 应用设置
sysctl -p
```

### 应用优化

```toml
# cgminer.toml 性能优化配置
[performance]
worker_threads = 4
work_queue_size = 1000
result_queue_size = 1000
batch_size = 100
```

## 故障排除

### 常见问题

1. **设备检测失败**
   - 检查硬件连接
   - 验证用户权限
   - 查看内核日志

2. **矿池连接失败**
   - 检查网络连接
   - 验证矿池地址和端口
   - 检查防火墙设置

3. **高温告警**
   - 检查风扇运行状态
   - 清理散热器
   - 降低频率和电压

### 日志分析

```bash
# 查看实时日志
sudo journalctl -u cgminer-rs -f

# 查看错误日志
sudo journalctl -u cgminer-rs -p err

# 查看特定时间段日志
sudo journalctl -u cgminer-rs --since "2024-01-01 00:00:00" --until "2024-01-01 23:59:59"
```

## 升级

### 在线升级

```bash
# 下载新版本
wget https://github.com/your-org/cgminer-rs/releases/latest/download/cgminer-rs-linux-aarch64.tar.gz

# 停止服务
sudo systemctl stop cgminer-rs

# 备份当前版本
sudo cp /usr/local/bin/cgminer-rs /usr/local/bin/cgminer-rs.backup

# 安装新版本
tar -xzf cgminer-rs-linux-aarch64.tar.gz
sudo cp cgminer-rs /usr/local/bin/
sudo chmod +x /usr/local/bin/cgminer-rs

# 启动服务
sudo systemctl start cgminer-rs

# 验证版本
cgminer-rs --version
```

### 回滚

```bash
# 停止服务
sudo systemctl stop cgminer-rs

# 恢复备份
sudo cp /usr/local/bin/cgminer-rs.backup /usr/local/bin/cgminer-rs

# 启动服务
sudo systemctl start cgminer-rs
```
