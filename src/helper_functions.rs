use crate::support::SupportThread;
use serenity::{
    client::Context,
    framework::standard::{CommandError, CommandResult},
    model::channel::Message,
    utils::Color,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

// ----------------
// Helper functions
// ----------------

// Helper function for fast and easy embed messages
pub async fn embed_msg(
    ctx: &Context,
    msg: &Message,
    text: &str,
    color: Color,
    autodelete: bool,
    autodelete_dur: Duration,
) -> CommandResult<Message> {
    let msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| e.description(text).color(color));
            m
        })
        .await?;

    if autodelete {
        tokio::time::sleep(autodelete_dur).await;
        msg.delete(ctx).await?;
    }

    Ok(msg)
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
            embed_msg(
                ctx,
                msg,
                "No reply sent in 60 seconds",
                Color::RED,
                false,
                Duration::from_secs(0),
            )
            .await?;
            return Err(CommandError::from(
                "No reply received for problem description",
            ));
        }
    };

    Ok(message)
}

pub async fn support_ticket_msg(
    ctx: &Context,
    msg: &Message,
    thread: &SupportThread,
) -> CommandResult {
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(format!("Support ticket [{}]", thread.incident_id))
                    .field("Title:", thread.incident_title.clone(), false)
                    .field(
                        "Status:",
                        format!(
                            "Solved: {}, Archived: {}",
                            thread.incident_solved, thread.thread_archived
                        ),
                        false,
                    )
                    .field("Timestamp:", thread.incident_time, false)
                    .field("Thread:", format!("<#{}>", thread.thread_id), false)
            })
        })
        .await?;
    Ok(())
}
