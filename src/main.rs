use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing, Router, Server,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wordgames=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState {
        global_message_tx: broadcast::channel(64).0,
        players: Mutex::new(HashSet::new()),
    });

    let app = Router::new()
        .route("/ws/anagram", routing::get(ws_anagram_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("Listening on {}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct AppState {
    global_message_tx: broadcast::Sender<String>,
    players: Mutex<HashSet<String>>,
}

async fn ws_anagram_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_anagram(socket, state))
}

async fn ws_anagram(ws_stream: WebSocket, state: Arc<AppState>) {
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Handle username registration
    let name = loop {
        if let Some(Ok(message)) = ws_rx.next().await {
            if let Message::Text(name) = message {
                let is_username_taken = {
                    let mut players = state.players.lock().unwrap();
                    let is_username_taken = players.contains(&name);

                    if !is_username_taken {
                        players.insert(name.clone());
                    }

                    is_username_taken
                };

                if !is_username_taken {
                    break name;
                }

                ws_tx
                    .send(Message::Text("Username already taken!".to_owned()))
                    .await
                    .unwrap();
            }
        }
    };

    // Now handle the messages.
    let mut global_message_rx = state.global_message_tx.subscribe();

    state
        .global_message_tx
        .send(format!("{} joined!", &name))
        .unwrap();

    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = global_message_rx.recv().await {
            if ws_tx.send(Message::Text(message)).await.is_err() {
                break;
            }
        }
    });

    let global_message_tx = state.global_message_tx.clone();
    let cloned_name = name.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(message))) = ws_rx.next().await {
            global_message_tx
                .send(format!("{}: {}", &cloned_name, message))
                .unwrap();
        }
    });

    // Handle exiting
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    state
        .global_message_tx
        .send(format!("{} left!", &name))
        .unwrap();
    state.players.lock().unwrap().remove(&name);
}
