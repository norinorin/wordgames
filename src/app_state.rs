use crate::{anagram::Anagram, handlers::CommandHandler};
use tokio::sync::Mutex;

pub struct AppState {
    pub anagram: Mutex<Anagram>,
    pub command_handler: CommandHandler,
}
