use crate::handlers::Context;

pub async fn handle_score(ctx: Context<'_>) {
    let anagram = ctx.state.anagram.lock().await;
    let score = anagram
        .player_to_points
        .get(ctx.author)
        .copied()
        .unwrap_or_default();
    ctx.tx
        .send(format!("@{}: Your score is {}.", ctx.author, score))
        .unwrap();
}
