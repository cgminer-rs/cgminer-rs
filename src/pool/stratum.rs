use crate::error::PoolError;
use crate::device::Work;
use crate::pool::Share;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{RwLock, Mutex};
use tokio::time::timeout;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Stratum 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumMessage {
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<StratumError>,
}

/// Stratum 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Stratum 客户端
pub struct StratumClient {
    /// 矿池URL
    url: String,
    /// 用户名
    username: String,
    /// 密码
    password: String,
    /// TCP连接 - 写入部分
    writer: Arc<Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>>,
    /// TCP连接 - 读取部分
    reader: Arc<Mutex<Option<tokio::net::tcp::OwnedReadHalf>>>,
    /// 连接状态
    connected: Arc<RwLock<bool>>,
    /// 订阅ID
    subscription_id: Arc<RwLock<Option<String>>>,
    /// Extra nonce 1
    extra_nonce1: Arc<RwLock<Option<String>>>,
    /// Extra nonce 2 大小
    extra_nonce2_size: Arc<RwLock<usize>>,
    /// 当前难度
    difficulty: Arc<RwLock<f64>>,
    /// 当前作业
    current_job: Arc<RwLock<Option<StratumJob>>>,
    /// 消息ID计数器
    message_id: Arc<RwLock<u64>>,
    /// 待处理的请求
    pending_requests: Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<StratumMessage>>>>,
}

/// Stratum 作业
#[derive(Debug, Clone)]
pub struct StratumJob {
    pub job_id: String,
    pub previous_hash: String,
    pub coinbase1: String,
    pub coinbase2: String,
    pub merkle_branches: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
}

impl StratumClient {
    /// 创建新的 Stratum 客户端
    pub async fn new(url: String, username: String, password: String) -> Result<Self, PoolError> {
        Ok(Self {
            url,
            username,
            password,
            writer: Arc::new(Mutex::new(None)),
            reader: Arc::new(Mutex::new(None)),
            connected: Arc::new(RwLock::new(false)),
            subscription_id: Arc::new(RwLock::new(None)),
            extra_nonce1: Arc::new(RwLock::new(None)),
            extra_nonce2_size: Arc::new(RwLock::new(4)),
            difficulty: Arc::new(RwLock::new(1.0)),
            current_job: Arc::new(RwLock::new(None)),
            message_id: Arc::new(RwLock::new(1)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 连接到矿池
    pub async fn connect(&mut self) -> Result<(), PoolError> {
        info!("Connecting to Stratum pool: {}", self.url);

        // 解析URL
        let url = self.url.strip_prefix("stratum+tcp://")
            .ok_or_else(|| PoolError::InvalidUrl { url: self.url.clone() })?;

        // 连接TCP
        let stream = match timeout(Duration::from_secs(10), TcpStream::connect(url)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                return Err(PoolError::ConnectionFailed {
                    url: self.url.clone(),
                    error: e.to_string(),
                });
            }
            Err(_) => {
                return Err(PoolError::Timeout { url: self.url.clone() });
            }
        };

        // 分离读写流
        let (reader, writer) = stream.into_split();
        *self.reader.lock().await = Some(reader);
        *self.writer.lock().await = Some(writer);
        *self.connected.write().await = true;

        // 启动消息处理循环
        self.start_message_loop().await?;

        // 发送订阅请求
        self.subscribe().await?;

        // 发送认证请求
        self.authorize().await?;

        info!("Successfully connected to Stratum pool");
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<(), PoolError> {
        info!("Disconnecting from Stratum pool");

        *self.connected.write().await = false;

        // 关闭读写流
        if let Some(reader) = self.reader.lock().await.take() {
            drop(reader);
        }
        if let Some(writer) = self.writer.lock().await.take() {
            drop(writer);
        }

        // 清理状态
        *self.subscription_id.write().await = None;
        *self.extra_nonce1.write().await = None;
        *self.current_job.write().await = None;
        self.pending_requests.write().await.clear();

        info!("Disconnected from Stratum pool");
        Ok(())
    }

    /// 订阅挖矿通知
    async fn subscribe(&self) -> Result<(), PoolError> {
        debug!("Sending mining.subscribe");

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.subscribe".to_string()),
            params: Some(json!(["cgminer-rs/1.0.0"])),
            result: None,
            error: None,
        };

        let response = self.send_request(message).await?;

        if let Some(result) = response.result {
            if let Some(array) = result.as_array() {
                if array.len() >= 2 {
                    // 解析订阅响应
                    if let Some(subscription_id) = array.get(1).and_then(|v| v.as_str()) {
                        *self.subscription_id.write().await = Some(subscription_id.to_string());
                    }

                    if array.len() >= 3 {
                        if let Some(extra_nonce1) = array.get(2).and_then(|v| v.as_str()) {
                            *self.extra_nonce1.write().await = Some(extra_nonce1.to_string());
                        }
                    }

                    if array.len() >= 4 {
                        if let Some(extra_nonce2_size) = array.get(3).and_then(|v| v.as_u64()) {
                            *self.extra_nonce2_size.write().await = extra_nonce2_size as usize;
                        }
                    }
                }
            }
        }

        debug!("Mining subscription successful");
        Ok(())
    }

    /// 认证
    async fn authorize(&self) -> Result<(), PoolError> {
        debug!("Sending mining.authorize");

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.authorize".to_string()),
            params: Some(json!([self.username, self.password])),
            result: None,
            error: None,
        };

        let response = self.send_request(message).await?;

        if let Some(result) = response.result {
            if result.as_bool() == Some(true) {
                debug!("Mining authorization successful");
                Ok(())
            } else {
                Err(PoolError::AuthenticationFailed { url: self.url.clone() })
            }
        } else if let Some(error) = response.error {
            Err(PoolError::StratumError {
                error_code: error.code,
                message: error.message,
            })
        } else {
            Err(PoolError::AuthenticationFailed { url: self.url.clone() })
        }
    }

    /// 提交份额
    pub async fn submit_share(&self, share: &Share) -> Result<bool, PoolError> {
        debug!("Submitting share: nonce={:08x}", share.nonce);

        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.submit".to_string()),
            params: Some(json!([
                self.username,
                share.job_id,
                share.extra_nonce2,
                format!("{:08x}", share.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                format!("{:08x}", share.nonce)
            ])),
            result: None,
            error: None,
        };

        let response = self.send_request(message).await?;

        if let Some(result) = response.result {
            Ok(result.as_bool().unwrap_or(false))
        } else if let Some(error) = response.error {
            Err(PoolError::ShareRejected { reason: error.message })
        } else {
            Ok(false)
        }
    }

    /// 获取工作
    pub async fn get_work(&self) -> Result<Work, PoolError> {
        let job = self.current_job.read().await;

        if let Some(job) = job.as_ref() {
            // 构造工作数据
            let work = self.build_work_from_job(job)?;
            Ok(work)
        } else {
            Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "No current job available".to_string(),
            })
        }
    }

    /// 从作业构造工作
    fn build_work_from_job(&self, job: &StratumJob) -> Result<Work, PoolError> {
        // 这里需要实现从 Stratum 作业到 Work 的转换
        // 为了简化，我们创建一个基本的工作结构

        let mut header = [0u8; 80];
        let target = [0u8; 32];

        // 解析版本
        if let Ok(version_bytes) = hex::decode(&job.version) {
            if version_bytes.len() >= 4 {
                header[0..4].copy_from_slice(&version_bytes[0..4]);
            }
        }

        // 解析前一个区块哈希
        if let Ok(prev_hash_bytes) = hex::decode(&job.previous_hash) {
            if prev_hash_bytes.len() >= 32 {
                header[4..36].copy_from_slice(&prev_hash_bytes[0..32]);
            }
        }

        // 解析时间
        if let Ok(time_bytes) = hex::decode(&job.ntime) {
            if time_bytes.len() >= 4 {
                header[68..72].copy_from_slice(&time_bytes[0..4]);
            }
        }

        // 解析难度目标
        if let Ok(bits_bytes) = hex::decode(&job.nbits) {
            if bits_bytes.len() >= 4 {
                header[72..76].copy_from_slice(&bits_bytes[0..4]);
            }
        }

        let difficulty = *tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.difficulty.read().await
            })
        });

        Ok(Work::new(
            job.job_id.clone(),
            target,
            header,
            difficulty,
        ))
    }

    /// 发送ping
    pub async fn ping(&self) -> Result<(), PoolError> {
        let message = StratumMessage {
            id: Some(self.next_message_id().await),
            method: Some("mining.ping".to_string()),
            params: None,
            result: None,
            error: None,
        };

        let _response = self.send_request(message).await?;
        Ok(())
    }

    /// 发送请求并等待响应
    async fn send_request(&self, message: StratumMessage) -> Result<StratumMessage, PoolError> {
        let message_id = message.id.unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();

        // 注册待处理请求
        self.pending_requests.write().await.insert(message_id, tx);

        // 发送消息
        self.send_message(message).await?;

        // 等待响应
        match timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(PoolError::ProtocolError {
                url: self.url.clone(),
                error: "Request cancelled".to_string(),
            }),
            Err(_) => {
                // 清理待处理请求
                self.pending_requests.write().await.remove(&message_id);
                Err(PoolError::Timeout { url: self.url.clone() })
            }
        }
    }

    /// 发送消息
    async fn send_message(&self, message: StratumMessage) -> Result<(), PoolError> {
        let json_str = serde_json::to_string(&message)
            .map_err(|e| PoolError::ProtocolError {
                url: self.url.clone(),
                error: format!("JSON serialization error: {}", e),
            })?;

        let mut writer_guard = self.writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.write_all(json_str.as_bytes()).await
                .map_err(|e| PoolError::ConnectionFailed {
                    url: self.url.clone(),
                    error: e.to_string(),
                })?;

            writer.write_all(b"\n").await
                .map_err(|e| PoolError::ConnectionFailed {
                    url: self.url.clone(),
                    error: e.to_string(),
                })?;

            writer.flush().await
                .map_err(|e| PoolError::ConnectionFailed {
                    url: self.url.clone(),
                    error: e.to_string(),
                })?;
        } else {
            return Err(PoolError::ConnectionFailed {
                url: self.url.clone(),
                error: "Not connected".to_string(),
            });
        }

        debug!("Sent: {}", json_str);
        Ok(())
    }

    /// 启动消息处理循环
    async fn start_message_loop(&self) -> Result<(), PoolError> {
        let reader = self.reader.clone();
        let connected = self.connected.clone();
        let pending_requests = self.pending_requests.clone();
        let current_job = self.current_job.clone();
        let difficulty = self.difficulty.clone();

        tokio::spawn(async move {
            // 获取读取流
            let reader_stream = {
                let mut reader_guard = reader.lock().await;
                reader_guard.take()
            };

            if let Some(reader_stream) = reader_stream {
                let mut buf_reader = BufReader::new(reader_stream);
                let mut line = String::new();

                while *connected.read().await {
                    line.clear();

                    match buf_reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            if let Ok(message) = serde_json::from_str::<StratumMessage>(&line.trim()) {
                                debug!("Received: {}", line.trim());

                                // 处理响应
                                if let Some(id) = message.id {
                                    let mut pending = pending_requests.write().await;
                                    if let Some(tx) = pending.remove(&id) {
                                        let _ = tx.send(message);
                                        continue;
                                    }
                                }

                                // 处理通知
                                if let Some(method) = &message.method {
                                    match method.as_str() {
                                        "mining.notify" => {
                                            // 处理新作业通知
                                            if let Some(params) = &message.params {
                                                if let Some(job) = Self::parse_job_notification(params) {
                                                    *current_job.write().await = Some(job);
                                                }
                                            }
                                        }
                                        "mining.set_difficulty" => {
                                            // 处理难度设置
                                            if let Some(params) = &message.params {
                                                if let Some(array) = params.as_array() {
                                                    if let Some(diff) = array.get(0).and_then(|v| v.as_f64()) {
                                                        *difficulty.write().await = diff;
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            debug!("Unknown method: {}", method);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading from stream: {}", e);
                            break;
                        }
                    }
                }

                *connected.write().await = false;
            }
        });

        Ok(())
    }

    /// 解析作业通知
    fn parse_job_notification(params: &Value) -> Option<StratumJob> {
        if let Some(array) = params.as_array() {
            if array.len() >= 9 {
                return Some(StratumJob {
                    job_id: array[0].as_str()?.to_string(),
                    previous_hash: array[1].as_str()?.to_string(),
                    coinbase1: array[2].as_str()?.to_string(),
                    coinbase2: array[3].as_str()?.to_string(),
                    merkle_branches: array[4].as_array()?
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    version: array[5].as_str()?.to_string(),
                    nbits: array[6].as_str()?.to_string(),
                    ntime: array[7].as_str()?.to_string(),
                    clean_jobs: array[8].as_bool().unwrap_or(false),
                });
            }
        }
        None
    }

    /// 获取下一个消息ID
    async fn next_message_id(&self) -> u64 {
        let mut id = self.message_id.write().await;
        *id += 1;
        *id
    }
}
