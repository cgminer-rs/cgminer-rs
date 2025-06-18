// CGMiner-RS 监控面板 JavaScript

class MiningDashboard {
    constructor() {
        this.updateInterval = 5000; // 5秒更新一次
        this.isOnline = false;
        this.lastUpdateTime = null;
        
        this.init();
    }

    init() {
        console.log('🚀 初始化挖矿监控面板');
        this.startAutoUpdate();
        this.updateData(); // 立即更新一次
    }

    startAutoUpdate() {
        setInterval(() => {
            this.updateData();
        }, this.updateInterval);
    }

    async updateData() {
        try {
            const response = await fetch('/api/dashboard');
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            
            const data = await response.json();
            this.renderDashboard(data);
            this.setOnlineStatus(true);
            this.lastUpdateTime = new Date();
            
        } catch (error) {
            console.error('❌ 获取数据失败:', error);
            this.setOnlineStatus(false);
        }
    }

    setOnlineStatus(online) {
        this.isOnline = online;
        const statusDot = document.getElementById('status-dot');
        const statusText = document.getElementById('status-text');
        
        if (online) {
            statusDot.className = 'status-dot online';
            statusText.textContent = '在线';
        } else {
            statusDot.className = 'status-dot offline';
            statusText.textContent = '离线';
        }
        
        // 更新最后更新时间
        const lastUpdateElement = document.getElementById('last-update');
        if (this.lastUpdateTime) {
            lastUpdateElement.textContent = this.formatTime(this.lastUpdateTime);
        }
    }

    renderDashboard(data) {
        // 更新概览卡片
        this.updateOverviewCards(data);
        
        // 更新设备状态
        this.updateDevices(data.devices);
        
        // 更新矿池状态
        this.updatePools(data.pools);
        
        // 更新统计信息
        this.updateStats(data.stats);
    }

    updateOverviewCards(data) {
        // 总算力
        const totalHashrate = document.getElementById('total-hashrate');
        if (data.mining) {
            totalHashrate.textContent = `${data.mining.total_hashrate.toFixed(2)} GH/s`;
        }

        // 份额统计
        if (data.mining) {
            document.getElementById('accepted-shares').textContent = data.mining.accepted_shares.toLocaleString();
            document.getElementById('rejected-shares').textContent = data.mining.rejected_shares.toLocaleString();
            document.getElementById('reject-rate').textContent = `${data.mining.reject_rate.toFixed(2)}%`;
        }

        // 系统状态
        if (data.system) {
            document.getElementById('temperature').textContent = `${data.system.temperature.toFixed(1)}°C`;
            document.getElementById('memory-usage').textContent = `${data.system.memory_usage.toFixed(1)}%`;
            document.getElementById('power-consumption').textContent = `${data.system.power_consumption.toFixed(1)}W`;
        }

        // 运行统计
        if (data.system) {
            document.getElementById('uptime').textContent = `${data.system.uptime_hours.toFixed(1)}小时`;
        }
        if (data.mining) {
            document.getElementById('active-devices').textContent = data.mining.active_devices;
            document.getElementById('efficiency').textContent = `${data.mining.efficiency.toFixed(2)} MH/J`;
        }
    }

    updateDevices(devices) {
        const devicesGrid = document.getElementById('devices-grid');
        
        if (!devices || devices.length === 0) {
            devicesGrid.innerHTML = '<div class="no-data">暂无设备数据</div>';
            return;
        }

        devicesGrid.innerHTML = devices.map(device => {
            const statusClass = this.getDeviceStatusClass(device);
            const statusText = this.getDeviceStatusText(device);
            
            return `
                <div class="device-card ${statusClass}">
                    <div class="device-header">
                        <span class="device-id">设备 #${device.device_id}</span>
                        <span class="device-status status-${statusClass}">${statusText}</span>
                    </div>
                    <div class="device-metrics">
                        <div class="metric-row">
                            <span class="metric-label">温度:</span>
                            <span class="metric-value">${device.temperature.toFixed(1)}°C</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">算力:</span>
                            <span class="metric-value">${device.hashrate.toFixed(2)} GH/s</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">功耗:</span>
                            <span class="metric-value">${device.power.toFixed(1)}W</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">错误率:</span>
                            <span class="metric-value">${device.error_rate.toFixed(2)}%</span>
                        </div>
                    </div>
                </div>
            `;
        }).join('');
    }

    updatePools(pools) {
        const poolsGrid = document.getElementById('pools-grid');
        
        if (!pools || pools.length === 0) {
            poolsGrid.innerHTML = '<div class="no-data">暂无矿池数据</div>';
            return;
        }

        poolsGrid.innerHTML = pools.map(pool => {
            const statusClass = pool.connected ? 'normal' : 'offline';
            const statusText = pool.connected ? '已连接' : '离线';
            const pingText = pool.ping_ms ? `${pool.ping_ms}ms` : '--';
            
            return `
                <div class="pool-card">
                    <div class="pool-header">
                        <span class="pool-id">矿池 #${pool.pool_id}</span>
                        <span class="pool-status status-${statusClass}">${statusText}</span>
                    </div>
                    <div class="pool-metrics">
                        <div class="metric-row">
                            <span class="metric-label">延迟:</span>
                            <span class="metric-value">${pingText}</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">接受份额:</span>
                            <span class="metric-value">${pool.accepted_shares.toLocaleString()}</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">拒绝份额:</span>
                            <span class="metric-value">${pool.rejected_shares.toLocaleString()}</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">连接时间:</span>
                            <span class="metric-value">${pool.uptime_hours.toFixed(1)}小时</span>
                        </div>
                    </div>
                </div>
            `;
        }).join('');
    }

    updateStats(stats) {
        if (!stats) return;
        
        document.getElementById('total-runtime').textContent = `${stats.total_runtime_hours.toFixed(1)}小时`;
        document.getElementById('total-shares').textContent = stats.total_shares.toLocaleString();
        document.getElementById('average-hashrate').textContent = `${stats.average_hashrate.toFixed(2)} GH/s`;
        document.getElementById('best-share').textContent = stats.best_share.toFixed(0);
        document.getElementById('hardware-errors').textContent = stats.hardware_errors.toLocaleString();
    }

    getDeviceStatusClass(device) {
        if (device.hashrate === 0) return 'error';
        if (device.temperature > 80) return 'warning';
        if (device.error_rate > 5) return 'warning';
        return 'normal';
    }

    getDeviceStatusText(device) {
        if (device.hashrate === 0) return '离线';
        if (device.temperature > 80) return '过热';
        if (device.error_rate > 5) return '高错误率';
        return '正常';
    }

    formatTime(date) {
        return date.toLocaleTimeString('zh-CN', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
        });
    }
}

// 页面加载完成后初始化
document.addEventListener('DOMContentLoaded', () => {
    new MiningDashboard();
});

// 添加一些实用功能
document.addEventListener('keydown', (e) => {
    // 按 R 键刷新数据
    if (e.key === 'r' || e.key === 'R') {
        location.reload();
    }
    
    // 按 F5 刷新页面
    if (e.key === 'F5') {
        e.preventDefault();
        location.reload();
    }
});

// 添加页面可见性检测，页面不可见时暂停更新
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        console.log('📱 页面隐藏，暂停更新');
    } else {
        console.log('📱 页面显示，恢复更新');
    }
});
