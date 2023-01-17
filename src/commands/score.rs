use crate::handlers::Context;

pub async fn handle_score(ctx: Context<'_>) {
    let anagram = ctx.state.anagram.lock().await;
    let score = anagram
        .player_to_points
        .get(ctx.author)
        .map(|x| *x)
        .unwrap_or_default();
    ctx.tx
        .send(format!("@{}: your score is {}", ctx.author, score))
        .unwrap();
}
