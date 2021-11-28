use serenity::{
    client::Context,
    framework::standard::{CommandError, CommandResult},
    model::channel::Message,
    utils::Color,
};
use std::{sync::Arc, time::Duration};

// ----------------
// Helper functions
// ----------------

// Helper function for fast and easy embed messages
pub async fn embed_msg(ctx: &Context, msg: &Message, text: &str, color: Color) -> CommandResult {
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| e.description(text).color(color));
            m
        })
        .await?;
    Ok(())
}

// Function for waiting for the author of msg to send a message
pub async fn wait_for_message(ctx: &Context, msg: &Message) -> CommandResult<Arc<Message>> {
    let message = match msg
        .author
        .await_reply(ctx)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(msg) => msg,
        None => {
            embed_msg(ctx, msg, "No reply sent in 60 seconds", Color::RED).await?;
            return Err(CommandError::from(
                "No reply received for problem description",
            ));
        }
    };

    Ok(message)
}
