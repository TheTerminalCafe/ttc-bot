use chrono::{DateTime, Utc};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        channel::Message,
        event::MessageUpdateEvent,
        guild::Member,
        id::{ChannelId, MessageId, UserId},
        prelude::User,
    },
    prelude::Mentionable,
    utils::{content_safe, Color, ContentSafeOptions},
};

use crate::typemap::{config::Config, types::PgPoolType};
use rand::seq::SliceRandom;

// Types for fetching/writing data from/to SQL database
struct CurrentIndex {
    current_id: i32,
}

#[allow(dead_code)] // A few of these paramteres are technically never read, but it is best that they are available in case they are needed
struct CachedMessage {
    id: i32,
    message_id: Option<i64>,
    channel_id: Option<i64>,
    user_id: Option<i64>,
    message_time: Option<DateTime<Utc>>,
    content: Option<String>,
    attachments: Option<String>,
}

// --------------------------------
// Functions for conveyance logging
// --------------------------------

// Store 500 most recent messages seen by this bot in a cache for informing when it had been
// deleted
pub async fn message(ctx: &Context, msg: &Message) {
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    let mut id = match sqlx::query_as!(
        CurrentIndex,
        r#"SELECT current_id FROM ttc_conveyance_state"#
    )
    .fetch_one(pool)
    .await
    {
        Ok(id) => id,
        Err(why) => {
            log::error!("Reading from database failed: {}", why);
            return;
        }
    };

    // Set the id to 0 to loop thru the message cache after reaching the end of the cache
    if id.current_id >= 500 {
        id.current_id = 0;
    }
    // Increment the id to move over to a new entry in the cache
    id.current_id += 1;

    // Write the message contents to the cache
    match sqlx::query!(
        r#"UPDATE ttc_message_cache SET message_id = $1, channel_id = $2, user_id = $3, message_time = $4, content = $5, attachments = $6 WHERE id = $7"#, 
        msg.id.0 as i64,
        msg.channel_id.0 as i64,
        msg.author.id.0 as i64,
        Utc::now(),
        msg.content_safe(ctx).await,
        msg.attachments.iter().map(|a| a.url.clone()).collect::<Vec<String>>().join(" "),
        id.current_id
    )
    .execute(pool)
    .await {
        Ok(_) => (),
        Err(why) => {
            log::error!("Writing to database failed: {}", why);
            return;
        }
    }

    match sqlx::query!(
        r#"UPDATE ttc_conveyance_state SET current_id = $1"#,
        id.current_id
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Writing to database failed: {}", why);
            return;
        }
    }
}

// Send logging messages when messages are deleted
pub async fn message_delete(ctx: &Context, channel_id: &ChannelId, deleted_message_id: &MessageId) {
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();
    let config = match Config::get_from_db(pool).await {
        Ok(config) => config,
        Err(why) => {
            log::error!("Error getting config from database: {}", why);
            return;
        }
    };

    // Get the cached message from the database
    let msg = match sqlx::query_as!(
        CachedMessage,
        r#"SELECT * FROM ttc_message_cache WHERE message_id = $1 AND channel_id = $2"#,
        deleted_message_id.0 as i64,
        channel_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(msg) => msg,
        Err(why) => {
            match why {
                sqlx::Error::RowNotFound => {
                    log::info!("Could not locate deleted message in database");
                }
                _ => log::error!("Error reading message from message cache database: {}", why),
            }
            return;
        }
    };

    // Get the user from either cache or rest api
    let user = match UserId(msg.user_id.unwrap() as u64).to_user(ctx).await {
        Ok(user) => user,
        Err(why) => {
            log::error!("Error getting user based on user id: {}", why);
            User::default()
        }
    };
    // Make sure both content and attachment strings are not empty as being empty would cause
    // errors when sending the embed
    let mut content = if msg.content.as_ref().unwrap() == "" {
        "None".to_string()
    } else {
        msg.content.unwrap()
    };
    let mut attachments = if msg.attachments.as_ref().unwrap() == "" {
        "None".to_string()
    } else {
        msg.attachments.unwrap()
    };

    content.truncate(1024);
    attachments.truncate(1024);

    match ChannelId(config.conveyance_channel as u64)
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Message deleted")
                    .author(|a| a.name(&user.name).icon_url(user.face()))
                    .color(Color::GOLD)
                    .field("User", user.tag(), true)
                    .field("UserId", user.id, true)
                    .field("Message sent at", msg.message_time.unwrap(), false)
                    .field("Channel", format!("<#{}>", msg.channel_id.unwrap()), true)
                    .field("Content", content, false)
                    .field("Attachments", attachments, false)
                    .timestamp(Utc::now())
            })
        })
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Failed to send message: {}", why);
            return;
        }
    }
}

// Send logging messages when a message is edited
pub async fn message_update(
    ctx: &Context,
    old: Option<Message>,
    new: Option<Message>,
    event: &MessageUpdateEvent,
) {
    let config = {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("Error getting config from database: {}", why);
                return;
            }
        }
    };

    // Make sure the channel isn't blacklisted from conveyance
    if config
        .conveyance_blacklisted_channels
        .contains(&(event.channel_id.0 as i64))
    {
        return;
    }

    // Create the embed outside the closures to allow for async calls
    let mut message_embed = CreateEmbed::default();
    message_embed.title("Message edited");
    message_embed.timestamp(Utc::now());
    message_embed.color(Color::DARK_GOLD);

    // Get the user info if it is available from the event
    match &event.author {
        Some(author) => {
            message_embed.field("User", author.tag(), true);
            message_embed.field("UserId", author.id, true);
        }
        None => {
            message_embed.field("User", "User tag not available", true);
            message_embed.field("UserId", "User id not available", true);
        }
    }

    // Add the channel embed here to preserve the proper
    message_embed.field("Channel", format!("<#{}>", &event.channel_id.0), false);

    // Make sure the contents actually have values
    match old {
        Some(old) => {
            let mut content_safe = old.content_safe(ctx).await;
            content_safe.truncate(1024);
            if content_safe == "" {
                content_safe = "None".to_string();
            }
            message_embed.field("Old", content_safe, false);
        }
        None => {
            message_embed.field("Old", "No old message content available", false);
        }
    }

    // Make sure the event is about the content being edited
    match &event.content {
        Some(content) => {
            // Check if the new message is available
            match new {
                Some(new) => {
                    log::debug!("Edited message content got based on provided `new` argument");

                    let mut content_safe = new.content_safe(ctx).await;
                    content_safe.truncate(1024);
                    if content_safe == "" {
                        content_safe = "None".to_string();
                    }
                    message_embed.field("New", content_safe, false);
                }
                // Try to fetch the new message from the api
                None => match event.channel_id.message(ctx, event.id).await {
                    Ok(new) => {
                        log::debug!("Edited message content got based on provided message got from the channel_id");

                        let mut content_safe = new.content_safe(ctx).await;
                        content_safe.truncate(1024);
                        if content_safe == "" {
                            content_safe = "None".to_string();
                        }
                        message_embed.field("New", content_safe, false);
                    }
                    // Fall back to the event in case all other methods fail
                    Err(why) => {
                        log::error!("Error getting message: {}", why);
                        log::debug!("Edited message content got based on raw event");

                        let mut content_safe =
                            content_safe(ctx, &content, &ContentSafeOptions::default()).await;
                        content_safe.truncate(1024);
                        if content_safe == "" {
                            content_safe = "None".to_string();
                        }
                        message_embed.field("New", content_safe, false);
                    }
                },
            }
        }
        None => {
            return;
        }
    }

    match ChannelId(config.conveyance_channel as u64)
        .send_message(ctx, |m| m.set_embed(message_embed))
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Error sending message: {}", why);
            return;
        }
    }
}

pub async fn guild_member_addition(ctx: &Context, new_member: &Member) {
    let config = {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("Error getting config from database: {}", why);
                return;
            }
        }
    };

    let welcome_message = config
        .welcome_messages
        .choose(&mut rand::thread_rng())
        .unwrap();
    let welcome_message = welcome_message.replace("%user%", &new_member.mention().to_string());

    match ChannelId(config.welcome_channel as u64)
        .send_message(ctx, |m| m.content(welcome_message))
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Error sending message: {}", why);
            return;
        }
    }

    match ChannelId(config.conveyance_channel as u64)
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("New member joined")
                    .color(Color::FOOYOO)
                    .field("User", new_member.user.tag(), true)
                    .field("UserId", new_member.user.id, true)
                    .field("Account created", new_member.user.created_at(), false)
            })
        })
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Error sending message: {}", why);
            return;
        }
    }
}

pub async fn guild_member_removal(ctx: &Context, user: &User, member: Option<Member>) {
    let config = {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("Error getting config from database: {}", why);
                return;
            }
        }
    };

    let joined_at = match member {
        Some(member) => match member.joined_at {
            Some(joined_at) => format!("{}", joined_at),
            None => "Join date not available".to_string(),
        },
        None => "Join date not available".to_string(),
    };

    match ChannelId(config.conveyance_channel as u64)
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Member left")
                    .color(Color::RED)
                    .field("User", user.tag(), true)
                    .field("UserId", user.id, true)
                    .field("Joined at", joined_at, false)
            })
        })
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Error sending message: {}", why),
    }
}
