use crate::{data::types::ShardManagerType, utils::helper_functions::embed_msg};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::channel::Message,
    utils::Color,
};

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
    let mut data = ctx.data.read().await;
    let shard_manager = data.get_mut::<ShardManagerType>().unwrap();

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Goodbye!"),
        Some("ttc-bot shutting down."),
        Some(Color::PURPLE),
        None,
    )
    .await?;
    shard_manager.lock().await.shutdown_all().await;
    Ok(())
}
