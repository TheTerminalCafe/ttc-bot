use poise::serenity_prelude;
// ----------------------
// General group commands
// ----------------------


#[poise::command(slash_command)]
pub async fn ping(ctx: crate::Context<'_>) -> Result<(), crate::Error> {
    ctx.say("pong").await?;

    Ok(())
}
