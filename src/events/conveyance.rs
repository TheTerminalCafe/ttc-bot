use crate::{get_config, types::Data, utils::helper_functions::format_datetime};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::*;

// Types for fetching/writing data from/to SQL database
struct CurrentIndex {
    current_id: i32,
}

#[allow(dead_code)] // A few of these parameters are technically never read, but it is best that they are available in case they are needed
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
pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    let pool = &data.pool;

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
        msg.content_safe(ctx),
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
pub async fn message_delete(
    ctx: &Context,
    channel_id: &ChannelId,
    deleted_message_id: &MessageId,
    data: &Data,
) {
    let config = get_config!(data);
    let pool = &data.pool;

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
            log::warn!("Error getting user based on user id: {}", why);
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

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("Message deleted")
                        .color(Color::GOLD)
                        .field("User", user.tag(), true)
                        .field("UserId", user.id, true)
                        .field(
                            "Message sent at",
                            format_datetime(&msg.message_time.unwrap()),
                            false,
                        )
                        .field("Channel", format!("<#{}>", msg.channel_id.unwrap()), true)
                        .field("Content", content.clone(), false)
                        .field("Attachments", attachments.clone(), false)
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
}

pub async fn message_delete_bulk(
    ctx: &Context,
    channel_id: &ChannelId,
    deleted_message_ids: &Vec<MessageId>,
    data: &Data,
) {
    for id in deleted_message_ids {
        message_delete(ctx, channel_id, id, data).await;
    }
}

// Send logging messages when a message is edited
pub async fn message_update(
    ctx: &Context,
    new: &Option<Message>,
    event: &MessageUpdateEvent,
    data: &Data,
) {
    let config = get_config!(data);
    // Make sure the channel isn't blacklisted from conveyance
    if config
        .conveyance_blacklisted_channels
        .contains(&(event.channel_id.0 as i64))
    {
        return;
    }

    let pool = &data.pool;

    // Create the embed outside the closures to allow for async calls
    let mut message_embed = CreateEmbed::default();
    message_embed.title("Message edited");
    message_embed.timestamp(Utc::now());
    message_embed.color(Color::DARK_GOLD);

    // Get the user info if it is available from the event
    match &event.author {
        Some(author) => {
            message_embed.field("User", author.tag(), true);
            message_embed.field("UserID", author.id, true);
        }
        None => {
            message_embed.field("User", "User tag not available", true);
            message_embed.field("UserID", "User id not available", true);
        }
    }

    // Add the channel embed here to preserve the proper
    message_embed.field("Channel", format!("<#{}>", &event.channel_id.0), false);

    // Get the cached message from the database
    let mut old_content = match sqlx::query_as!(
        CachedMessage,
        r#"SELECT * FROM ttc_message_cache WHERE message_id = $1 AND channel_id = $2"#,
        event.id.0 as i64,
        event.channel_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(msg) => match msg.content {
            Some(content) => {
                if content.len() > 0 {
                    content
                } else {
                    "None".to_string()
                }
            }
            None => "Not available.".to_string(),
        },
        Err(why) => {
            match why {
                sqlx::Error::RowNotFound => {
                    log::info!("Could not locate deleted message in database");
                }
                _ => log::error!("Error reading message from message cache database: {}", why),
            }
            "Not available.".to_string()
        }
    };

    old_content.truncate(1024);

    message_embed.field("Old", old_content, false);

    // Make sure the event is about the content being edited
    let new_content = match &event.content {
        Some(content) => {
            // Check if the new message is available
            match new {
                Some(new) => {
                    log::debug!("Edited message content got based on provided `new` argument");

                    let mut content_safe = new.content_safe(ctx);
                    content_safe.truncate(1024);
                    if content_safe == "" {
                        content_safe = "None".to_string();
                    }
                    content_safe
                }
                // Try to fetch the new message from the api
                None => match event.channel_id.message(ctx, event.id).await {
                    Ok(new) => {
                        log::debug!("Edited message content got based on provided message got from the channel_id");

                        let mut content_safe = new.content_safe(ctx);
                        content_safe.truncate(1024);
                        if content_safe == "" {
                            content_safe = "None".to_string();
                        }
                        content_safe
                    }
                    // Fall back to the event in case all other methods fail
                    Err(why) => {
                        log::warn!("Error getting message: {}", why);

                        let mut content_safe =
                            content_safe(ctx, &content, &ContentSafeOptions::default(), &[]);
                        content_safe.truncate(1024);
                        if content_safe == "" {
                            content_safe = "None".to_string();
                        }
                        content_safe
                    }
                },
            }
        }
        None => {
            return;
        }
    };

    message_embed.field("New", &new_content, false);

    match sqlx::query!(
        r#"UPDATE ttc_message_cache SET content = $1 WHERE message_id = $2"#,
        new_content,
        event.id.0 as i64
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Error updating message cache: {}", why);
            return;
        }
    }

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| m.set_embed(message_embed.clone()))
            .await
        {
            Ok(_) => (),
            Err(why) => {
                log::error!("Error sending message: {}", why);
                return;
            }
        }
    }
}

pub async fn guild_member_addition(ctx: &Context, new_member: &Member, data: &Data) {
    let config = get_config!(data);

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("New member joined")
                        .color(Color::FOOYOO)
                        .field("User", new_member.user.tag(), true)
                        .field("UserID", new_member.user.id, true)
                        .field(
                            "Account created",
                            format_datetime(&new_member.user.created_at()),
                            false,
                        )
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
}

pub async fn guild_member_removal(
    ctx: &Context,
    user: &User,
    member: &Option<Member>,
    data: &Data,
) {
    let config = get_config!(data);

    let joined_at = match member {
        Some(member) => match member.joined_at {
            Some(joined_at) => format_datetime(&joined_at),
            None => "Join date not available".to_string(),
        },
        None => "Join date not available".to_string(),
    };

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("Member left")
                        .color(Color::RED)
                        .field("User", user.tag(), true)
                        .field("UserID", user.id, true)
                        .field("Joined at", joined_at.clone(), false)
                })
            })
            .await
        {
            Ok(_) => (),
            Err(why) => log::error!("Error sending message: {}", why),
        }
    }
}
pub async fn guild_ban_addition(ctx: &Context, banned_user: &User, data: &Data) {
    let config = get_config!(data);

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("User banned.")
                        .field("User", banned_user.tag(), true)
                        .field("UserID", banned_user.id, true)
                        .color(Color::DARK_RED)
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
}

pub async fn guild_ban_removal(ctx: &Context, unbanned_user: &User, data: &Data) {
    let config = get_config!(data);

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("User unbanned")
                        .field("User", unbanned_user.tag(), true)
                        .field("UserID", unbanned_user.id, true)
                        .color(Color::FOOYOO)
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
}

pub async fn guild_member_update(ctx: &Context, old: &Option<Member>, new: &Member, data: &Data) {
    let config = get_config!(data);

    let (old_nickname, old_roles, old_timeouted) = match old {
        Some(old) => {
            let old_nickname = match old.nick.clone() {
                Some(nick) => nick,
                None => "None".to_string(),
            };
            let old_timeouted = match old.communication_disabled_until {
                Some(comm_disabled) => {
                    if comm_disabled.unix_timestamp() < Timestamp::now().unix_timestamp() {
                        false
                    } else {
                        true
                    }
                }
                None => false,
            };

            (old_nickname, old.roles.clone(), Some(old_timeouted))
        }
        None => ("N/A".to_string(), Vec::new(), None),
    };

    let new_nickname = match new.nick.clone() {
        Some(nick) => nick,
        None => "None".to_string(),
    };
    let new_roles = new.roles.clone();
    let new_timeouted = match new.communication_disabled_until {
        Some(comm_disabled) => {
            if comm_disabled.unix_timestamp() < Timestamp::now().unix_timestamp() {
                false
            } else {
                true
            }
        }
        None => false,
    };

    // Make sure it is only the values displayed that have changed
    if !(old_nickname != new_nickname
        || old_roles != new_roles
        || match old_timeouted {
            Some(old_timeouted) => {
                if old_timeouted == new_timeouted {
                    false
                } else {
                    true
                }
            }
            None => false,
        })
    {
        log::debug!("User updated, but no logging done");
        return;
    }

    let mut old_roles_string = String::new();
    let mut new_roles_string = String::new();

    for role in old_roles {
        old_roles_string.push_str(&format!("<@&{}>, ", role));
    }
    if old_roles_string.len() == 0 {
        old_roles_string = "None or N/A".to_string();
    } else {
        old_roles_string.pop();
        old_roles_string.pop();
    }

    for role in new_roles {
        new_roles_string.push_str(&format!("<@&{}>, ", role));
    }
    if new_roles_string.len() == 0 {
        new_roles_string = "None".to_string();
    } else {
        new_roles_string.pop();
        new_roles_string.pop();
    }

    for channel in &config.conveyance_channels {
        match ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title("User updated")
                        .field("User", new.user.tag(), true)
                        .field("UserID", new.user.id, true)
                        .field("Timed out", new_timeouted, false)
                        .field("Old nickname", &old_nickname, true)
                        .field("New nickname", &new_nickname, true)
                        .field("Old roles", &old_roles_string, false)
                        .field("New roles", &new_roles_string, false)
                        .color(Color::ORANGE)
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
}
