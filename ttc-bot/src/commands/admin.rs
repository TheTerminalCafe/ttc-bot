// --------------------
// Admin group commands
// --------------------

use poise::serenity_prelude::Color;

use crate::types;

#[poise::command(prefix_command, slash_command, owners_only)]
pub async fn shutdown(ctx: types::Context<'_>) -> Result<(), types::Error> {
    ctx.send(|m| m.embed(|e| e.title("Goodbye!").color(Color::PURPLE)))
        .await?;

    ctx.framework()
        .shard_manager()
        .lock()
        .await
        .shutdown_all()
        .await;

    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn register(ctx: types::Context<'_>) -> Result<(), types::Error> {
    log::info!("Registering slash commands");
    poise::builtins::register_application_commands(ctx, false).await?;
    Ok(())
}
