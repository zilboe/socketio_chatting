use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    routing::{delete, get, post},
};
use tokio::sync::{RwLock, mpsc};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use socketioxide::{
    SocketIo,
    adapter::Adapter,
    extract::{AckSender, Data, SocketRef},
    socket::DisconnectReason,
};
use std::{hash::Hash, sync::Arc, time::Duration};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Clone, Debug)]
struct AppState {
    client: Arc<RwLock<Vec<String>>>
}

impl AppState {
    fn new() -> Self {
        AppState {
            client: Arc::new(RwLock::new(Vec::new()))
        }
    }
}

#[tokio::main]
async fn main() {
    let ip = Ipv4Addr::new(127, 0, 0, 1);
    let port = 9010;

    let app = AppState::new();

    let (layer, io) = SocketIo::new_layer();

    let io_app = app.clone();
    io.ns("/",  async move |s: SocketRef, Data::<Value>(_msg)| {
        let app_clone = io_app.clone();
        let client = s.id.to_string();
        tokio::spawn( async move {
            info!("new client connect {}", client);
            let app_closure_clone = app_clone.clone();
            let mut app_closure_client_list = app_closure_clone.client.write().await;
            app_closure_client_list.push(client.clone());
        });

        s.on("message", async move |s: SocketRef, Data::<Value>(msg)| {
            let spwan_s = s.clone();
            let spwan_msg = msg.clone();
            tokio::spawn(async move {
                info!("new client {} message {}", spwan_s.id.to_string(), spwan_msg.to_string());
            });

            let ack_message = msg.to_string() + "Hello World";
            s.emit("message", &ack_message).ok();
        });

        let app_disconnect = io_app.clone();
        s.on_disconnect(async move |s: SocketRef| {
            let app_disconnect_closure = app_disconnect.clone();
            let client = s.id.to_string();
            tokio::spawn(async move {
                let app_client_closure = app_disconnect_closure.clone();
                let mut app_client_list = app_client_closure.client.write().await;
                if let Some(pos) = app_client_list.iter().position(|x| *x == client) {
                    app_client_list.remove(pos);
                }
            });
        });
    });

    let static_file = ServeDir::new("static");

    let route = Router::new().fallback_service(static_file).layer(layer);
    let app_clones = app.clone();
    tokio::spawn(async move {
        loop {
            println!("current socketio client is {:?}", app_clones);
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
    let addr = SocketAddr::new(IpAddr::V4(ip), port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, route.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
