use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::{app_state::AppState, server_message::chat, server_message::ServerMessage};

pub async fn ws_anagram_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_anagram(socket, state))
}

async fn ws_anagram(ws_stream: WebSocket, state: Arc<AppState>) {
    tracing::debug!("A player entered! Waiting for username entry.");
    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    ws_tx
        .send(Message::Text(chat!(
            "Hey there, please enter your name to proceed! Type /help for help."
        )))
        .await
        .unwrap();
    let name = loop {
        if let Some(Ok(Message::Text(name))) = ws_rx.next().await {
            if name.is_empty() {
                continue;
            }

            if !name.chars().next().unwrap().is_ascii_alphabetic() {
                ws_tx
                    .send(Message::Text(chat!(
                        "Usernames can only start with ascii letters."
                    )))
                    .await
                    .unwrap();
                continue;
            }
            let mut anagram = state.anagram.lock().await;

            if anagram.insert_player(&name) {
                break name;
            }

            ws_tx
                .send(Message::Text(chat!("Username already taken!")))
                .await
                .unwrap();
        } else {
            tracing::debug!("A player left before entering their username!");
            return;
        }
    };

    let tx = state.anagram.lock().await.tx.clone();
    let mut global_message_rx = tx.subscribe();

    tracing::debug!("{} joined!", &name);
    tx.send(chat!("{} joined!", &name)).unwrap();

    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = global_message_rx.recv().await {
            if ws_tx.send(Message::Text(message)).await.is_err() {
                break;
            }
        }
    });

    let global_message_tx = tx.clone();
    let cloned_name = name.clone();
    let cloned_state = state.clone();

    let mut recv_task = tokio::spawn(async move {
        handle_ws_recv(cloned_state, global_message_tx, cloned_name, &mut ws_rx).await;
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    tracing::debug!("{} left!", &name);
    tx.send(chat!("{} left!", &name)).unwrap();
    state.anagram.lock().await.remove_player(&name);
}

async fn handle_ws_recv(
    app_state: Arc<AppState>,
    global_message_tx: broadcast::Sender<String>,
    name: String,
    ws_rx: &mut SplitStream<WebSocket>,
) {
    while let Some(Ok(Message::Text(message))) = ws_rx.next().await {
        if message.is_empty() {
            continue;
        }

        tracing::debug!("{}: {}", &name, &message);

        if app_state.anagram.lock().await.guess(&name, &message).await {
            continue;
        }

        global_message_tx
            .send(chat!("{}: {}", &name, &message))
            .unwrap();

        app_state
            .command_handler
            .handle(&app_state, &global_message_tx, &name, &message)
            .await;
    }
}
