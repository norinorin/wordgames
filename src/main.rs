use std::{net::SocketAddr, sync::Arc};

use axum::{routing, Router, Server};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod anagram;
mod app_state;
mod commands;
mod handlers;
#[macro_use]
mod server_message;
use anagram::Anagram;
use app_state::AppState;
use handlers::CommandHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wordgames=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let command_handler = CommandHandler::default()
        .callback(
            "/start",
            Box::new(|ctx| Box::pin(commands::handle_start(ctx))),
        )
        .callback(
            "/score",
            Box::new(|ctx| Box::pin(commands::handle_score(ctx))),
        )
        .callback(
            "/help",
            Box::new(|ctx| Box::pin(commands::handle_help(ctx))),
        );

    let state = Arc::new(AppState {
        anagram: Mutex::new(Anagram::new(true).await?),
        command_handler,
    });

    let app = Router::new()
        .route("/ws/anagram", routing::get(handlers::ws_anagram_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Listening on {}", addr);
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}
