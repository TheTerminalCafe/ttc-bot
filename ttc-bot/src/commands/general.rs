use crate::types::{Context, Error};
// ----------------------
// General group commands
// ----------------------

#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong").await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn bump(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong").await?;

    Ok(())
}
