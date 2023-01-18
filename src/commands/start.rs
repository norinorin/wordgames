use crate::{
    handlers::Context,
    server_message::{chat, ServerMessage},
};

pub async fn handle_start(ctx: Context<'_>) {
    let mut anagram = ctx.state.anagram.lock().await;
    let mut duration = 30u32;
    if let Some(number) = ctx.message.split_ascii_whitespace().nth(1) {
        match number.parse::<u32>() {
            Ok(number) => {
                duration = number;
            }
            _ => {
                ctx.tx
                    .send(chat!(
                        "@{}: Failed to parse {number:?} as integer.",
                        ctx.author
                    ))
                    .unwrap();
                return;
            }
        }
    }

    anagram.start(duration).await;
}
