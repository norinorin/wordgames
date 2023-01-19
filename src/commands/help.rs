use crate::{
    handlers::Context,
    server_message::{chat, ServerMessage},
};

pub async fn handle_help(ctx: Context<'_>) {
    ctx.tx
        .send(chat!(
            r#"@{}: 
Commands:

- /help
Sends this message.
- /start (timeout_seconds, defaults to 30s)
Starts an anagram game.
- /score
Shows your score (currently arbitrary).
"#,
            ctx.author
        ))
        .unwrap();
}
