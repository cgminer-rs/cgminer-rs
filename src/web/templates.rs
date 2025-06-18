//! Web模板

/// 生成设备状态HTML
pub fn device_status_html(device_id: u32, online: bool, temperature: f32, hashrate: f64) -> String {
    let status_class = if online { "status-online" } else { "status-offline" };
    let status_text = if online { "在线" } else { "离线" };
    
    format!(
        r#"
        <div class="device-card">
            <h4>设备 {}</h4>
            <div class="device-status">
                <span class="status-indicator {}"></span>
                <span>{}</span>
            </div>
            <div class="device-metrics">
                <div class="metric">
                    <span class="metric-label">温度:</span>
                    <span class="metric-value">{:.1}°C</span>
                </div>
                <div class="metric">
                    <span class="metric-label">算力:</span>
                    <span class="metric-value">{:.2} GH/s</span>
                </div>
            </div>
        </div>
        "#,
        device_id, status_class, status_text, temperature, hashrate
    )
}

/// 生成矿池状态HTML
pub fn pool_status_html(pool_id: u32, connected: bool, ping: u32, accepted_shares: u64) -> String {
    let status_class = if connected { "status-online" } else { "status-offline" };
    let status_text = if connected { "已连接" } else { "未连接" };
    
    format!(
        r#"
        <div class="pool-card">
            <h4>矿池 {}</h4>
            <div class="pool-status">
                <span class="status-indicator {}"></span>
                <span>{}</span>
            </div>
            <div class="pool-metrics">
                <div class="metric">
                    <span class="metric-label">延迟:</span>
                    <span class="metric-value">{} ms</span>
                </div>
                <div class="metric">
                    <span class="metric-label">接受份额:</span>
                    <span class="metric-value">{}</span>
                </div>
            </div>
        </div>
        "#,
        pool_id, status_class, status_text, ping, accepted_shares
    )
}

/// 生成告警HTML
pub fn alert_html(title: &str, description: &str, severity: &str, timestamp: &str) -> String {
    let severity_class = match severity.to_lowercase().as_str() {
        "critical" => "alert-critical",
        "warning" => "alert-warning",
        "info" => "alert-info",
        _ => "alert-info",
    };
    
    format!(
        r#"
        <div class="alert-item {}">
            <div class="alert-header">
                <span class="alert-title">{}</span>
                <span class="alert-time">{}</span>
            </div>
            <div class="alert-description">{}</div>
        </div>
        "#,
        severity_class, title, timestamp, description
    )
}

/// 生成CSS样式
pub fn get_css() -> &'static str {
    r#"
    .device-card, .pool-card {
        background: white;
        border-radius: 6px;
        padding: 15px;
        margin: 10px 0;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        border-left: 4px solid #007bff;
    }
    
    .device-card h4, .pool-card h4 {
        margin: 0 0 10px 0;
        color: #333;
    }
    
    .device-status, .pool-status {
        margin-bottom: 10px;
    }
    
    .device-metrics, .pool-metrics {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 10px;
    }
    
    .metric {
        display: flex;
        justify-content: space-between;
    }
    
    .metric-label {
        color: #666;
    }
    
    .metric-value {
        font-weight: bold;
        color: #333;
    }
    
    .alert-item {
        background: white;
        border-radius: 6px;
        padding: 15px;
        margin: 10px 0;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    
    .alert-critical {
        border-left: 4px solid #dc3545;
    }
    
    .alert-warning {
        border-left: 4px solid #ffc107;
    }
    
    .alert-info {
        border-left: 4px solid #17a2b8;
    }
    
    .alert-header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 8px;
    }
    
    .alert-title {
        font-weight: bold;
        color: #333;
    }
    
    .alert-time {
        color: #666;
        font-size: 14px;
    }
    
    .alert-description {
        color: #555;
        line-height: 1.4;
    }
    "#
}

/// 生成JavaScript代码
pub fn get_javascript() -> &'static str {
    r#"
    class CGMinerDashboard {
        constructor() {
            this.refreshInterval = 30000; // 30秒
            this.init();
        }
        
        init() {
            this.setupEventListeners();
            this.startAutoRefresh();
            this.refreshData();
        }
        
        setupEventListeners() {
            const refreshBtn = document.getElementById('refreshBtn');
            if (refreshBtn) {
                refreshBtn.addEventListener('click', () => this.refreshData());
            }
        }
        
        startAutoRefresh() {
            setInterval(() => this.refreshData(), this.refreshInterval);
        }
        
        async fetchData(url) {
            try {
                const response = await fetch(url);
                if (!response.ok) {
                    throw new Error(`HTTP error! status: ${response.status}`);
                }
                return await response.json();
            } catch (error) {
                console.error('Failed to fetch data from', url, ':', error);
                return null;
            }
        }
        
        async refreshData() {
            console.log('Refreshing dashboard data...');
            
            // 更新系统状态
            await this.updateSystemStatus();
            
            // 更新指标
            await this.updateMetrics();
            
            // 更新设备状态
            await this.updateDevices();
            
            // 更新矿池状态
            await this.updatePools();
            
            // 更新告警
            await this.updateAlerts();
            
            console.log('Dashboard data refreshed');
        }
        
        async updateSystemStatus() {
            const data = await this.fetchData('/api/status');
            if (data) {
                const statusElement = document.getElementById('systemStatus');
                if (statusElement) {
                    const isRunning = data.state === 'Running';
                    statusElement.innerHTML = `
                        <span class="status-indicator ${isRunning ? 'status-online' : 'status-offline'}"></span>
                        <span>${data.state || '未知'}</span>
                    `;
                }
            }
        }
        
        async updateMetrics() {
            // 更新系统指标
            const systemData = await this.fetchData('/api/metrics/system');
            if (systemData) {
                this.updateElement('systemTemp', systemData.temperature, '°C', 1);
                this.updateElement('memoryUsage', systemData.memory_usage, '%', 1);
                this.updateElement('cpuUsage', systemData.cpu_usage, '%', 1);
            }
            
            // 更新挖矿指标
            const miningData = await this.fetchData('/api/metrics/mining');
            if (miningData) {
                this.updateElement('totalHashrate', miningData.total_hashrate, 'GH/s', 2);
                this.updateElement('acceptedShares', miningData.accepted_shares, 'shares');
                this.updateElement('rejectedShares', miningData.rejected_shares, 'shares');
            }
        }
        
        updateElement(id, value, unit = '', decimals = 0) {
            const element = document.getElementById(id);
            if (element && value !== undefined && value !== null) {
                const formattedValue = typeof value === 'number' ? 
                    value.toFixed(decimals) : value;
                element.textContent = formattedValue;
            }
        }
        
        async updateDevices() {
            const data = await this.fetchData('/api/metrics/devices');
            if (data && data.devices) {
                const container = document.getElementById('devicesContainer');
                if (container) {
                    // 这里可以动态更新设备列表
                    console.log('Device data:', data.devices);
                }
            }
        }
        
        async updatePools() {
            const data = await this.fetchData('/api/metrics/pools');
            if (data && data.pools) {
                const container = document.getElementById('poolsContainer');
                if (container) {
                    // 这里可以动态更新矿池列表
                    console.log('Pool data:', data.pools);
                }
            }
        }
        
        async updateAlerts() {
            const data = await this.fetchData('/api/alerts');
            if (data) {
                const container = document.getElementById('alertsContainer');
                if (container) {
                    // 这里可以动态更新告警列表
                    console.log('Alert data:', data);
                }
            }
        }
    }
    
    // 页面加载完成后初始化仪表板
    document.addEventListener('DOMContentLoaded', () => {
        new CGMinerDashboard();
    });
    "#
}
