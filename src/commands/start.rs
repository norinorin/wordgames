use crate::handlers::Context;

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
                    .send(format!(
                        "To {}: Invalid start match format! Not a number.",
                        ctx.author
                    ))
                    .unwrap();
                return;
            }
        }
    }

    anagram.start(duration);
}
