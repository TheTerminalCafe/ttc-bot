use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::channel::Message,
};

#[group]
#[commands(ping)]
struct General;

// ----------------------
// General group commands
// ----------------------

#[command]
#[description("Ping!")]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply_ping(ctx, "pong").await?;

    Ok(())
}
