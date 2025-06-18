//! Web处理器

use crate::monitoring::MonitoringSystem;
use std::sync::Arc;
use warp::Reply;
use serde_json::json;
use tracing::debug;

/// 首页处理器
pub async fn index() -> Result<impl Reply, warp::Rejection> {
    debug!("Serving index page");

    let html = r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CGMiner-RS 监控面板</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            padding: 20px;
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 2px solid #eee;
        }
        .header h1 {
            color: #333;
            margin: 0;
        }
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .metric-card {
            background: #f8f9fa;
            border-radius: 6px;
            padding: 20px;
            border-left: 4px solid #007bff;
        }
        .metric-card h3 {
            margin: 0 0 15px 0;
            color: #333;
        }
        .metric-value {
            font-size: 24px;
            font-weight: bold;
            color: #007bff;
            margin-bottom: 5px;
        }
        .metric-unit {
            color: #666;
            font-size: 14px;
        }
        .status-indicator {
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 8px;
        }
        .status-online { background-color: #28a745; }
        .status-offline { background-color: #dc3545; }
        .status-warning { background-color: #ffc107; }
        .refresh-btn {
            background: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
        }
        .refresh-btn:hover {
            background: #0056b3;
        }
        .api-links {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #eee;
        }
        .api-links h3 {
            color: #333;
        }
        .api-links a {
            display: inline-block;
            margin: 5px 10px 5px 0;
            padding: 8px 12px;
            background: #6c757d;
            color: white;
            text-decoration: none;
            border-radius: 4px;
            font-size: 14px;
        }
        .api-links a:hover {
            background: #545b62;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 CGMiner-RS 监控面板</h1>
            <p>实时监控您的挖矿设备状态和性能</p>
            <button class="refresh-btn" onclick="refreshData()">🔄 刷新数据</button>
        </div>

        <div class="metrics-grid" id="metricsGrid">
            <div class="metric-card">
                <h3>📊 系统状态</h3>
                <div id="systemStatus">
                    <span class="status-indicator status-offline"></span>
                    <span>加载中...</span>
                </div>
            </div>

            <div class="metric-card">
                <h3>⚡ 总算力</h3>
                <div class="metric-value" id="totalHashrate">--</div>
                <div class="metric-unit">GH/s</div>
            </div>

            <div class="metric-card">
                <h3>🌡️ 系统温度</h3>
                <div class="metric-value" id="systemTemp">--</div>
                <div class="metric-unit">°C</div>
            </div>

            <div class="metric-card">
                <h3>💾 内存使用</h3>
                <div class="metric-value" id="memoryUsage">--</div>
                <div class="metric-unit">%</div>
            </div>

            <div class="metric-card">
                <h3>✅ 接受份额</h3>
                <div class="metric-value" id="acceptedShares">--</div>
                <div class="metric-unit">shares</div>
            </div>

            <div class="metric-card">
                <h3>❌ 拒绝份额</h3>
                <div class="metric-value" id="rejectedShares">--</div>
                <div class="metric-unit">shares</div>
            </div>
        </div>

        <div class="api-links">
            <h3>📡 API 接口</h3>
            <a href="/api/status" target="_blank">系统状态</a>
            <a href="/api/metrics/system" target="_blank">系统指标</a>
            <a href="/api/metrics/mining" target="_blank">挖矿指标</a>
            <a href="/api/metrics/devices" target="_blank">设备指标</a>
            <a href="/api/metrics/pools" target="_blank">矿池指标</a>
            <a href="/api/stats/performance" target="_blank">性能统计</a>
            <a href="/api/alerts" target="_blank">告警信息</a>
            <a href="/metrics" target="_blank">Prometheus指标</a>
        </div>
    </div>

    <script>
        async function fetchData(url) {
            try {
                const response = await fetch(url);
                return await response.json();
            } catch (error) {
                console.error('Failed to fetch data:', error);
                return null;
            }
        }

        async function refreshData() {
            // 获取系统状态
            const status = await fetchData('/api/status');
            if (status) {
                const statusElement = document.getElementById('systemStatus');
                const isRunning = status.state === 'Running';
                statusElement.innerHTML = `
                    <span class="status-indicator ${isRunning ? 'status-online' : 'status-offline'}"></span>
                    <span>${status.state || '未知'}</span>
                `;
            }

            // 获取系统指标
            const systemMetrics = await fetchData('/api/metrics/system');
            if (systemMetrics) {
                document.getElementById('systemTemp').textContent =
                    systemMetrics.temperature ? systemMetrics.temperature.toFixed(1) : '--';
                document.getElementById('memoryUsage').textContent =
                    systemMetrics.memory_usage ? systemMetrics.memory_usage.toFixed(1) : '--';
            }

            // 获取挖矿指标
            const miningMetrics = await fetchData('/api/metrics/mining');
            if (miningMetrics) {
                document.getElementById('totalHashrate').textContent =
                    miningMetrics.total_hashrate ? miningMetrics.total_hashrate.toFixed(2) : '--';
                document.getElementById('acceptedShares').textContent =
                    miningMetrics.accepted_shares || '--';
                document.getElementById('rejectedShares').textContent =
                    miningMetrics.rejected_shares || '--';
            }
        }

        // 页面加载时刷新数据
        document.addEventListener('DOMContentLoaded', refreshData);

        // 每30秒自动刷新
        setInterval(refreshData, 30000);
    </script>
</body>
</html>
    "#;

    Ok(warp::reply::html(html))
}

/// API状态处理器
pub async fn api_status(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting system status");

    let state = monitoring.get_state().await;
    let response = json!({
        "state": format!("{:?}", state),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(warp::reply::json(&response))
}

/// API系统指标处理器
pub async fn api_system_metrics(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting system metrics");

    match monitoring.get_system_metrics().await {
        Some(metrics) => Ok(warp::reply::json(&metrics)),
        None => {
            let response = json!({
                "error": "No system metrics available"
            });
            Ok(warp::reply::json(&response))
        }
    }
}

/// API挖矿指标处理器
pub async fn api_mining_metrics(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting mining metrics");

    match monitoring.get_mining_metrics().await {
        Some(metrics) => Ok(warp::reply::json(&metrics)),
        None => {
            let response = json!({
                "error": "No mining metrics available"
            });
            Ok(warp::reply::json(&response))
        }
    }
}

/// API设备指标处理器
pub async fn api_device_metrics(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting device metrics");

    let mut devices = serde_json::Map::new();

    // 获取前10个设备的指标
    for device_id in 0..10u32 {
        if let Some(metrics) = monitoring.get_device_metrics(device_id).await {
            devices.insert(device_id.to_string(), serde_json::to_value(metrics).unwrap());
        }
    }

    let response = json!({
        "devices": devices
    });

    Ok(warp::reply::json(&response))
}

/// API矿池指标处理器
pub async fn api_pool_metrics(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting pool metrics");

    let mut pools = serde_json::Map::new();

    // 获取前5个矿池的指标
    for pool_id in 0..5u32 {
        if let Some(metrics) = monitoring.get_pool_metrics(pool_id).await {
            pools.insert(pool_id.to_string(), serde_json::to_value(metrics).unwrap());
        }
    }

    let response = json!({
        "pools": pools
    });

    Ok(warp::reply::json(&response))
}

/// API性能统计处理器
pub async fn api_performance_stats(monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting performance stats");

    let stats = monitoring.get_performance_stats().await;
    Ok(warp::reply::json(&stats))
}

/// API告警处理器
pub async fn api_alerts(_monitoring: Arc<MonitoringSystem>) -> Result<impl Reply, warp::Rejection> {
    debug!("API: Getting alerts");

    // 这里应该从告警管理器获取活跃告警
    // 目前返回模拟数据
    let response = json!({
        "active_alerts": [],
        "alert_count": 0,
        "last_updated": chrono::Utc::now().to_rfc3339()
    });

    Ok(warp::reply::json(&response))
}
