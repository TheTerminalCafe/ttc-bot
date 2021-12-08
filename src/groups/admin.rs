use crate::{data::types::ShardManagerType, utils::helper_functions::embed_msg};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        CommandError, CommandResult,
    },
    model::channel::Message,
    utils::Color,
};
use std::time::Duration;

#[group]
#[prefixes("admin")]
#[owners_only]
#[commands(shutdown)]
struct Admin;

// --------------------
// Admin group commands
// --------------------

#[command]
async fn shutdown(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let shard_manager = match data.get_mut::<ShardManagerType>() {
        Some(shard_manager) => shard_manager.lock(),
        None => {
            embed_msg(
                ctx,
                &msg.channel_id,
                "**Error**: Shutdown failed",
                Color::RED,
                false,
                Duration::from_secs(0),
            )
            .await?;
            return Err(CommandError::from("No shard manager in data!"));
        }
    };
    embed_msg(
        ctx,
        &msg.channel_id,
        "**Goodbye!**",
        Color::PURPLE,
        false,
        Duration::from_secs(0),
    )
    .await?;
    shard_manager.await.shutdown_all().await;
    Ok(())
}
