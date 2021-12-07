use chrono::Utc;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::Message,
        event::MessageUpdateEvent,
        id::{ChannelId, MessageId},
    },
    utils::{content_safe, ContentSafeOptions},
};

use crate::ConveyanceChannelType;

// --------------------------------
// Functions for conveyance logging
// --------------------------------

// FIXME Currently non-functional, needs a message cache database to be functional
pub async fn message_delete(ctx: &Context, channel_id: &ChannelId, deleted_message_id: &MessageId) {
    // Get the conveyance channel id from the data typemap
    let conveyance_channel_id = {
        let data = ctx.data.read().await;
        ChannelId(*data.get::<ConveyanceChannelType>().unwrap())
    };

    let msg = match ctx.cache.message(channel_id, deleted_message_id).await {
        Some(msg) => msg,
        None => {
            match conveyance_channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Message deleted")
                            .description("Message content not found in cache")
                            .field("Id", deleted_message_id, true)
                    })
                })
                .await
            {
                Ok(_) => (),
                Err(why) => println!("An error occurred: {}", why),
            }
            return;
        }
    };

    let mut content_safe = msg.content_safe(ctx).await;
    content_safe.truncate(1024);

    let name = msg
        .author_nick(ctx)
        .await
        .unwrap_or(msg.author.name.clone());

    match conveyance_channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Message deleted")
                    .author(|a| a.name(name).icon_url(msg.author.face()))
                    .field("Username", msg.author.tag(), true)
                    .field("UserId", msg.author.id, true)
                    .field("Content", content_safe, false)
                    .timestamp(Utc::now())
            })
        })
        .await
    {
        Ok(_) => (),
        Err(why) => println!("An error occurred: {}", why),
    }
}

pub async fn message_update(ctx: &Context, old: Option<Message>, event: &MessageUpdateEvent) {
    // Get the conveyance channel id from the data typemap
    let conveyance_channel_id = {
        let data = ctx.data.read().await;
        ChannelId(*data.get::<ConveyanceChannelType>().unwrap())
    };

    // Create the embed outside the closures to allow for async calls
    let mut message_embed = CreateEmbed::default();
    message_embed.title("Message edited");
    message_embed.timestamp(Utc::now());

    match old {
        Some(old) => {
            let mut content_safe = old.content_safe(ctx).await;
            content_safe.truncate(1024);
            message_embed.field("Old", content_safe, false);
        }
        None => {
            message_embed.field("Old", "No old message content available", false);
        }
    }
    match &event.content {
        Some(content) => {
            let mut content_safe =
                content_safe(ctx, &content, &ContentSafeOptions::default()).await;
            content_safe.truncate(1024);
            message_embed.field("New", content_safe, false);
        }
        None => {
            message_embed.field("New", "No new message content available", false);
        }
    }

    conveyance_channel_id
        .send_message(ctx, |m| m.set_embed(message_embed))
        .await
        .unwrap();
}
