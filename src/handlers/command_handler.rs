use futures_util::Future;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::broadcast;

use crate::app_state::AppState;

type Callback =
    Box<dyn Fn(Context<'_>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync + '_>> + Send + Sync>;

#[derive(Default)]
pub struct CommandHandler {
    callbacks: HashMap<String, Callback>,
}

impl CommandHandler {
    pub fn callback(mut self, prefix: &str, callback: Callback) -> Self {
        self.callbacks.insert(prefix.to_owned(), callback);
        self
    }

    pub async fn handle(
        &self,
        state: &Arc<AppState>,
        tx: &broadcast::Sender<String>,
        author: &String,
        message: &String,
    ) -> bool {
        if !self.is_valid_command(message) {
            return false;
        }

        if let Some(callback) = self
            .callbacks
            .get(message.split_ascii_whitespace().next().unwrap())
        {
            callback(Context {
                state,
                tx,
                author,
                message,
            })
            .await;
        }

        true
    }

    fn is_valid_command(&self, message: &str) -> bool {
        if !message.starts_with('/') {
            return false;
        }

        if let Some(command) = message.split_ascii_whitespace().next() {
            return self.callbacks.contains_key(command);
        }

        false
    }
}

pub struct Context<'a> {
    pub state: &'a Arc<AppState>,
    pub tx: &'a broadcast::Sender<String>,
    pub author: &'a String,
    pub message: &'a String,
}
