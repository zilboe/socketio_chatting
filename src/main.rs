
use tokio::sync::{RwLock, mpsc};
use axum::{
    extract::{Path as AxumPath, State},
    routing::{delete, get, post},
    Json, Router,
};

use std::{collections::HashMap, net::{IpAddr, Ipv4Addr, SocketAddr}};
use std::sync::Arc;
use socketioxide::{
    extract::{Data, SocketRef, AckSender},
    SocketIo,
    socket::DisconnectReason
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
};
use uuid::Uuid;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
use tracing::info;
struct AppState {

}

fn create_socket_handler(io: Arc<SocketIo>) -> impl Fn(SocketRef) + Send + Sync + 'static {
    // 捕获外部的io实例
    move |socket: SocketRef| {
        let io = io.clone(); // 克隆Arc
        
        let client_id = socket.id.to_string();
        println!("Client connected: {}", client_id);
        
        // 保存连接信息（可选，但不是必须的）
        // io实例已经内部管理了连接
        
        socket.on("message", {
            let io = io.clone();
            move |socket: SocketRef, Data(data): Data<Value>| {
                let io = io.clone();
                async move {
                    info!(?data, "Received event:");
                    
                    // 使用io实例主动向其他客户端发送消息
                    // 例如：广播给所有客户端（除了发送者）
                    io.broadcast()
                        .except(vec![socket.id.to_string()])
                        .emit("broadcast-message", &data);
                        
                    socket.emit("message-back", &data).ok();
                }
            }
        });
        
        // 断开连接处理
        socket.on_disconnect({
            let io = io.clone();
            async move |socket_ref: SocketRef, reason: DisconnectReason| {
                let client_id = socket_ref.id.to_string();
                info!("Client {} disconnected: {:?}", client_id, reason);
                
                // 使用io实例通知其他客户端
                let _ = io.emit("user-left", &json!({
                    "client_id": client_id,
                    "reason": reason.to_string()
                })).await;
            }
        });
    }
}

#[tokio::main]
async fn main() {
    let ip = Ipv4Addr::new(127,0,0,1);
    let port = 9010;

    let (layer, io) = SocketIo::new_layer();

    let io_arc = Arc::new(io);

    let handler = create_socket_handler(io_arc.clone());

    io_arc.ns("/", handler);

    let static_file = ServeDir::new("static");

    let route = Router::new()
    .fallback_service(static_file)
    .layer(layer);

    let addr = SocketAddr::new(IpAddr::V4(ip), port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, route.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();

    
}
