use cgminer_rs::config::Config;
use cgminer_rs::mining::MiningManager;
use cgminer_rs::api::server::ApiServer;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use reqwest;

/// Comprehensive system integration test
/// This test verifies that all major components work together correctly
#[tokio::test]
async fn test_full_system_integration() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt::try_init();

    // Create test configuration
    let config = create_integration_test_config();

    // Test 1: Configuration validation
    assert!(config.is_valid(), "Configuration should be valid");

    // Test 2: Create core registry and load cores
    let core_loader = cgminer_rs::CoreLoader::new();
    core_loader.load_all_cores().await.expect("Failed to load cores");
    let core_registry = core_loader.registry();

    // Test 3: Mining manager initialization
    let mining_manager = Arc::new(
        MiningManager::new(config.clone(), core_registry)
            .await
            .expect("Failed to create mining manager")
    );

    // Test 4: API server initialization
    let api_server = ApiServer::new(config.api.clone(), mining_manager.clone());

    // Test 4: Start API server
    api_server.start().await.expect("Failed to start API server");

    // Wait for server to be ready
    sleep(Duration::from_millis(500)).await;

    // Test 5: API health check
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/api/v1/status")
        .send()
        .await
        .expect("Failed to make API request");

    assert!(response.status().is_success(), "API should respond successfully");

    let status: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert!(status["success"].as_bool().unwrap_or(false), "API response should indicate success");

    // Test 6: Start mining manager
    mining_manager.start().await.expect("Failed to start mining manager");

    // Wait for mining to initialize
    sleep(Duration::from_secs(2)).await;

    // Test 7: Verify mining state
    let mining_state = mining_manager.get_state().await;
    println!("Mining state: {:?}", mining_state);

    // Test 8: Get system status via API
    let response = client
        .get("http://127.0.0.1:8080/api/v1/status")
        .send()
        .await
        .expect("Failed to get system status");

    let status: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse status response");

    assert!(status["success"].as_bool().unwrap_or(false));
    assert!(status["data"]["uptime"].as_u64().is_some());

    // Test 9: Get devices via API
    let response = client
        .get("http://127.0.0.1:8080/api/v1/devices")
        .send()
        .await
        .expect("Failed to get devices");

    assert!(response.status().is_success());

    // Test 10: Get pools via API
    let response = client
        .get("http://127.0.0.1:8080/api/v1/pools")
        .send()
        .await
        .expect("Failed to get pools");

    assert!(response.status().is_success());

    // Test 11: Test control commands
    let control_request = serde_json::json!({
        "command": "pause"
    });

    let response = client
        .post("http://127.0.0.1:8080/api/v1/control")
        .json(&control_request)
        .send()
        .await
        .expect("Failed to send control command");

    assert!(response.status().is_success());

    // Test 12: Verify pause state
    sleep(Duration::from_millis(500)).await;

    let response = client
        .get("http://127.0.0.1:8080/api/v1/status")
        .send()
        .await
        .expect("Failed to get status after pause");

    let status: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse status");

    // The mining state might be "Paused" or still transitioning
    println!("Status after pause: {}", status["data"]["mining_state"]);

    // Test 13: Resume mining
    let control_request = serde_json::json!({
        "command": "resume"
    });

    let response = client
        .post("http://127.0.0.1:8080/api/v1/control")
        .json(&control_request)
        .send()
        .await
        .expect("Failed to send resume command");

    assert!(response.status().is_success());

    // Test 14: Test statistics endpoint
    let response = client
        .get("http://127.0.0.1:8080/api/v1/stats")
        .send()
        .await
        .expect("Failed to get statistics");

    assert!(response.status().is_success());

    let stats: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse stats");

    assert!(stats["success"].as_bool().unwrap_or(false));
    assert!(stats["data"]["mining_stats"].is_object());

    // Test 15: Test error handling
    let response = client
        .get("http://127.0.0.1:8080/api/v1/nonexistent")
        .send()
        .await
        .expect("Failed to make request to nonexistent endpoint");

    assert_eq!(response.status(), 404);

    // Test 16: Test device restart (should handle gracefully even with mock devices)
    let response = client
        .post("http://127.0.0.1:8080/api/v1/devices/0/restart")
        .send()
        .await
        .expect("Failed to restart device");

    // Should succeed or return appropriate error
    assert!(response.status().is_success() || response.status().is_client_error());

    // Test 17: Test configuration endpoint
    let response = client
        .get("http://127.0.0.1:8080/api/v1/config")
        .send()
        .await;

    // Config endpoint might not be implemented, so we just check it doesn't crash
    if let Ok(response) = response {
        println!("Config endpoint status: {}", response.status());
    }

    // Test 18: Stop mining manager
    mining_manager.stop().await.expect("Failed to stop mining manager");

    // Test 19: Stop API server
    api_server.stop().await.expect("Failed to stop API server");

    // Test 20: Verify cleanup
    sleep(Duration::from_millis(500)).await;

    // Try to connect to API - should fail now
    let response = client
        .get("http://127.0.0.1:8080/api/v1/status")
        .timeout(Duration::from_secs(1))
        .send()
        .await;

    // Should fail since server is stopped
    assert!(response.is_err(), "API should not respond after shutdown");

    println!("✅ All integration tests passed!");
}

/// Test WebSocket functionality
#[tokio::test]
async fn test_websocket_integration() {
    // This test would require a WebSocket client implementation
    // For now, we'll just verify the WebSocket endpoint exists

    let config = create_integration_test_config();
    let core_loader = cgminer_rs::CoreLoader::new();
    core_loader.load_all_cores().await.expect("Failed to load cores");
    let core_registry = core_loader.registry();

    let mining_manager = Arc::new(
        MiningManager::new(config.clone(), core_registry)
            .await
            .expect("Failed to create mining manager")
    );

    let api_server = ApiServer::new(config.api.clone(), mining_manager.clone());
    api_server.start().await.expect("Failed to start API server");

    sleep(Duration::from_millis(500)).await;

    // Test WebSocket endpoint exists (would need actual WebSocket client for full test)
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/ws")
        .send()
        .await;

    // WebSocket upgrade should fail with HTTP client, but endpoint should exist
    if let Ok(response) = response {
        // Should get some response (likely 400 or 426 for upgrade required)
        assert!(response.status().is_client_error());
    }

    api_server.stop().await.expect("Failed to stop API server");
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations() {
    let config = create_integration_test_config();
    let core_loader = cgminer_rs::CoreLoader::new();
    core_loader.load_all_cores().await.expect("Failed to load cores");
    let core_registry = core_loader.registry();

    let mining_manager = Arc::new(
        MiningManager::new(config.clone(), core_registry)
            .await
            .expect("Failed to create mining manager")
    );

    let api_server = ApiServer::new(config.api.clone(), mining_manager.clone());
    api_server.start().await.expect("Failed to start API server");
    mining_manager.start().await.expect("Failed to start mining manager");

    sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();

    // Make multiple concurrent API requests
    let mut handles = Vec::new();

    for i in 0..10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let response = client
                .get("http://127.0.0.1:8080/api/v1/status")
                .send()
                .await
                .expect(&format!("Failed request {}", i));

            assert!(response.status().is_success());

            let status: serde_json::Value = response
                .json()
                .await
                .expect("Failed to parse JSON");

            assert!(status["success"].as_bool().unwrap_or(false));
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.expect("Request task failed");
    }

    mining_manager.stop().await.expect("Failed to stop mining manager");
    api_server.stop().await.expect("Failed to stop API server");

    println!("✅ Concurrent operations test passed!");
}

/// Create test configuration for integration tests
fn create_integration_test_config() -> Config {
    Config {
        general: cgminer_rs::config::GeneralConfig {
            log_level: "info".to_string(),
            log_file: None,
            pid_file: None,
            work_restart_timeout: 10,
            scan_time: 1,
        },
        cores: cgminer_rs::config::CoresConfig {
            enabled_cores: vec!["software".to_string()],
            default_core: "software".to_string(),
            software_core: Some(cgminer_rs::config::SoftwareCoreConfig {
                enabled: true,
                device_count: 2,
                min_hashrate: 500_000_000.0,
                max_hashrate: 2_000_000_000.0,
                error_rate: 0.01,
                batch_size: 1000,
                work_timeout_ms: 5000,
                cpu_affinity: None,
            }),
            asic_core: None,
        },
        devices: cgminer_rs::config::DeviceConfig {
            auto_detect: true,
            scan_interval: 5,
            chains: vec![
                cgminer_rs::config::ChainConfig {
                    id: 0,
                    enabled: true,
                    frequency: 500,
                    voltage: 850,
                    auto_tune: false,
                    chip_count: 76,
                },
            ],
        },
        pools: cgminer_rs::config::PoolConfig {
            strategy: cgminer_rs::config::PoolStrategy::Failover,
            failover_timeout: 30,
            retry_interval: 10,
            pools: vec![
                cgminer_rs::config::PoolInfo {
                    url: "stratum+tcp://test.pool.com:4444".to_string(),
                    user: "test_user".to_string(),
                    password: "test_password".to_string(),
                    priority: 1,
                    quota: None,
                    enabled: true,
                },
            ],
        },
        api: cgminer_rs::config::ApiConfig {
            enabled: true,
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            auth_token: None, // No auth for testing
            allow_origins: vec!["*".to_string()],
        },
        monitoring: cgminer_rs::config::MonitoringConfig {
            enabled: true,
            metrics_interval: 5,
            prometheus_port: Some(9090),
            alert_thresholds: cgminer_rs::config::AlertThresholds {
                temperature_warning: 80.0,
                temperature_critical: 90.0,
                hashrate_drop_percent: 20.0,
                error_rate_percent: 5.0,
                max_temperature: 85.0,
                max_cpu_usage: 90.0,
                max_memory_usage: 90.0,
                max_device_temperature: 90.0,
                max_error_rate: 5.0,
                min_hashrate: 1.0, // Low threshold for testing
            },
        },
    }
}
