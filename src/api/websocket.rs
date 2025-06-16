use crate::api::{AppState, WebSocketMessage};
use crate::mining::MiningEvent;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// WebSocket 连接管理器
pub struct WebSocketManager {
    /// 活跃连接
    connections: Arc<RwLock<std::collections::HashMap<Uuid, WebSocketConnection>>>,
    /// 广播发送器
    broadcast_sender: broadcast::Sender<WebSocketMessage>,
}

/// WebSocket 连接
pub struct WebSocketConnection {
    /// 连接ID
    id: Uuid,
    /// 订阅的事件类型
    subscriptions: Arc<RwLock<HashSet<String>>>,
    /// 消息发送器
    sender: Arc<Mutex<Option<futures_util::stream::SplitSink<WebSocket, Message>>>>,
    /// 连接时间
    connected_at: std::time::SystemTime,
    /// 最后活动时间
    last_activity: Arc<RwLock<std::time::SystemTime>>,
}

impl WebSocketManager {
    /// 创建新的 WebSocket 管理器
    pub fn new() -> Self {
        let (broadcast_sender, _) = broadcast::channel(1000);
        
        Self {
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
            broadcast_sender,
        }
    }
    
    /// 添加连接
    pub async fn add_connection(&self, connection: WebSocketConnection) {
        let id = connection.id;
        self.connections.write().await.insert(id, connection);
        info!("WebSocket connection added: {}", id);
    }
    
    /// 移除连接
    pub async fn remove_connection(&self, id: Uuid) {
        self.connections.write().await.remove(&id);
        info!("WebSocket connection removed: {}", id);
    }
    
    /// 广播消息
    pub async fn broadcast(&self, message: WebSocketMessage) {
        if let Err(e) = self.broadcast_sender.send(message) {
            debug!("Failed to broadcast WebSocket message: {}", e);
        }
    }
    
    /// 发送消息到特定连接
    pub async fn send_to_connection(&self, connection_id: Uuid, message: WebSocketMessage) {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&connection_id) {
            connection.send_message(message).await;
        }
    }
    
    /// 获取连接数量
    pub async fn get_connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// 获取连接统计
    pub async fn get_connection_stats(&self) -> WebSocketStats {
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut total_subscriptions = 0;
        
        for connection in connections.values() {
            total_subscriptions += connection.subscriptions.read().await.len();
        }
        
        WebSocketStats {
            total_connections,
            total_subscriptions,
            uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0)),
        }
    }
    
    /// 清理断开的连接
    pub async fn cleanup_connections(&self) {
        let mut connections = self.connections.write().await;
        let mut to_remove = Vec::new();
        
        for (id, connection) in connections.iter() {
            if connection.is_disconnected().await {
                to_remove.push(*id);
            }
        }
        
        for id in to_remove {
            connections.remove(&id);
            info!("Cleaned up disconnected WebSocket connection: {}", id);
        }
    }
}

impl WebSocketConnection {
    /// 创建新的 WebSocket 连接
    pub fn new(
        id: Uuid,
        sender: futures_util::stream::SplitSink<WebSocket, Message>,
    ) -> Self {
        Self {
            id,
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            sender: Arc::new(Mutex::new(Some(sender))),
            connected_at: std::time::SystemTime::now(),
            last_activity: Arc::new(RwLock::new(std::time::SystemTime::now())),
        }
    }
    
    /// 发送消息
    pub async fn send_message(&self, message: WebSocketMessage) {
        if let Ok(json) = serde_json::to_string(&message) {
            let mut sender_guard = self.sender.lock().await;
            if let Some(sender) = sender_guard.as_mut() {
                if let Err(e) = sender.send(Message::Text(json)).await {
                    warn!("Failed to send WebSocket message: {}", e);
                    // 连接可能已断开，移除发送器
                    *sender_guard = None;
                }
            }
        }
    }
    
    /// 订阅事件
    pub async fn subscribe(&self, events: Vec<String>) {
        let mut subscriptions = self.subscriptions.write().await;
        for event in events {
            subscriptions.insert(event);
        }
        self.update_activity().await;
    }
    
    /// 取消订阅事件
    pub async fn unsubscribe(&self, events: Vec<String>) {
        let mut subscriptions = self.subscriptions.write().await;
        for event in events {
            subscriptions.remove(&event);
        }
        self.update_activity().await;
    }
    
    /// 检查是否订阅了特定事件
    pub async fn is_subscribed(&self, event: &str) -> bool {
        self.subscriptions.read().await.contains(event)
    }
    
    /// 更新活动时间
    pub async fn update_activity(&self) {
        *self.last_activity.write().await = std::time::SystemTime::now();
    }
    
    /// 检查连接是否断开
    pub async fn is_disconnected(&self) -> bool {
        self.sender.lock().await.is_none()
    }
    
    /// 获取连接信息
    pub async fn get_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: self.id,
            connected_at: self.connected_at,
            last_activity: *self.last_activity.read().await,
            subscriptions: self.subscriptions.read().await.clone(),
        }
    }
}

/// WebSocket 处理器
pub struct WebSocketHandler {
    manager: Arc<WebSocketManager>,
    mining_manager: Arc<crate::mining::MiningManager>,
}

impl WebSocketHandler {
    /// 创建新的 WebSocket 处理器
    pub fn new(mining_manager: Arc<crate::mining::MiningManager>) -> Self {
        Self {
            manager: Arc::new(WebSocketManager::new()),
            mining_manager,
        }
    }
    
    /// 处理 WebSocket 升级
    pub async fn handle_upgrade(
        ws: WebSocketUpgrade,
        State(state): State<AppState>,
    ) -> Response {
        ws.on_upgrade(move |socket| Self::handle_socket(socket, state))
    }
    
    /// 处理 WebSocket 连接
    async fn handle_socket(socket: WebSocket, state: AppState) {
        let connection_id = Uuid::new_v4();
        info!("New WebSocket connection: {}", connection_id);
        
        let (sender, mut receiver) = socket.split();
        let connection = WebSocketConnection::new(connection_id, sender);
        
        // 创建处理器
        let handler = WebSocketHandler::new(state.mining_manager.clone());
        
        // 添加连接到管理器
        handler.manager.add_connection(connection).await;
        
        // 启动心跳任务
        let heartbeat_manager = handler.manager.clone();
        let heartbeat_id = connection_id;
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                let connections = heartbeat_manager.connections.read().await;
                if let Some(connection) = connections.get(&heartbeat_id) {
                    connection.send_message(WebSocketMessage::Ping).await;
                } else {
                    break; // 连接已移除
                }
            }
        });
        
        // 订阅挖矿事件
        let mut mining_events = state.mining_manager.subscribe_events();
        let event_manager = handler.manager.clone();
        let event_id = connection_id;
        tokio::spawn(async move {
            while let Ok(mining_event) = mining_events.recv().await {
                let ws_message = Self::convert_mining_event(mining_event);
                
                let connections = event_manager.connections.read().await;
                if let Some(connection) = connections.get(&event_id) {
                    if connection.is_subscribed("mining_events").await {
                        connection.send_message(ws_message).await;
                    }
                } else {
                    break; // 连接已移除
                }
            }
        });
        
        // 处理接收到的消息
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                        handler.handle_message(connection_id, ws_message).await;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed: {}", connection_id);
                    break;
                }
                Ok(Message::Pong(_)) => {
                    // 更新活动时间
                    let connections = handler.manager.connections.read().await;
                    if let Some(connection) = connections.get(&connection_id) {
                        connection.update_activity().await;
                    }
                }
                Err(e) => {
                    warn!("WebSocket error for connection {}: {}", connection_id, e);
                    break;
                }
                _ => {}
            }
        }
        
        // 移除连接
        handler.manager.remove_connection(connection_id).await;
    }
    
    /// 处理 WebSocket 消息
    async fn handle_message(&self, connection_id: Uuid, message: WebSocketMessage) {
        match message {
            WebSocketMessage::Subscribe { events } => {
                let connections = self.manager.connections.read().await;
                if let Some(connection) = connections.get(&connection_id) {
                    connection.subscribe(events).await;
                }
            }
            WebSocketMessage::Unsubscribe { events } => {
                let connections = self.manager.connections.read().await;
                if let Some(connection) = connections.get(&connection_id) {
                    connection.unsubscribe(events).await;
                }
            }
            WebSocketMessage::Ping => {
                self.manager
                    .send_to_connection(connection_id, WebSocketMessage::Pong)
                    .await;
            }
            _ => {
                // 其他消息类型暂不处理
            }
        }
    }
    
    /// 转换挖矿事件为 WebSocket 消息
    fn convert_mining_event(event: MiningEvent) -> WebSocketMessage {
        let event_type = event.event_type().to_string();
        let data = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
        
        WebSocketMessage::MiningEvent {
            event: event_type,
            data,
        }
    }
}

/// WebSocket 统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebSocketStats {
    pub total_connections: usize,
    pub total_subscriptions: usize,
    pub uptime: Duration,
}

/// 连接信息
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub connected_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
    pub subscriptions: HashSet<String>,
}

/// WebSocket 升级处理函数
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    WebSocketHandler::handle_upgrade(ws, State(state)).await
}
