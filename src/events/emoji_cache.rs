use crate::{types::Data, utils::emoji_cache::EmojiCache};
use poise::serenity_prelude::{
    ChannelId, Context, GuildId, Message, MessageId, MessageUpdateEvent,
};

/// The event to account for message deletions in emoji caching
pub async fn message_delete(
    ctx: &Context,
    guild_id: &Option<GuildId>,
    channel_id: &ChannelId,
    deleted_message_id: &MessageId,
    data: &Data,
) {
    // Make sure a cache refresh is not running
    if EmojiCache::is_running() {
        return;
    }

    let cache = match sqlx::query!(
        r#"SELECT * FROM ttc_emoji_cache_channels WHERE channel_id = $1"#,
        channel_id.0 as i64
    )
    .fetch_one(&data.pool)
    .await
    {
        Ok(cache) => cache,
        Err(why) => match why {
            sqlx::Error::RowNotFound => return,
            _ => {
                log::error!("Unable to get cached channel from DB: {}", why);
                return;
            }
        },
    };
    let msg = match sqlx::query!(
        r#"SELECT * FROM ttc_message_cache WHERE message_id = $1"#,
        deleted_message_id.0 as i64
    )
    .fetch_one(&data.pool)
    .await
    {
        Ok(msg) => msg,
        Err(why) => match why {
            sqlx::Error::RowNotFound => {
                log::warn!("Deleted message not found in database, emoji cache may be inaccurate.");
                return;
            }
            _ => {
                log::error!("Error getting message from database: {}", why);
                return;
            }
        },
    };
    // If the deleted message was sent before the latest cache message
    if msg.message_time.unwrap().timestamp() < cache.timestamp_unix {
        let mut emoji_cache = EmojiCache::new(&data.pool);
        let emojis = match guild_id.unwrap().emojis(ctx).await {
            Ok(emojis) => emojis,
            Err(why) => {
                log::error!("can't get emojis from guild: {}", why);
                return;
            }
        };
        for emoji in emojis {
            if msg
                .content
                .as_ref()
                .unwrap()
                .contains(&format!("<:{}:", emoji.name))
            {
                match emoji_cache
                    .decrease_emoji_count(msg.user_id.unwrap() as u64, emoji.name, 1)
                    .await
                {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("error decreasing the emoji count: {}", why);
                        return;
                    }
                }
            }
        }
        match emoji_cache
            .decrease_message_count(msg.user_id.unwrap() as u64, 1)
            .await
        {
            Ok(_) => (),
            Err(why) => {
                log::error!("error decreasing the message count: {}", why);
                return;
            }
        }
    }
}

pub async fn message_update(
    ctx: &Context,
    new: &Option<Message>,
    event: &MessageUpdateEvent,
    data: &Data,
) {
    // Make sure a cache refresh is not running
    if EmojiCache::is_running() {
        return;
    }
    // Get the emoji list of the guild
    let emoji_list = match match event.guild_id {
        Some(guild_id) => guild_id,
        None => return,
    }
    .emojis(ctx)
    .await
    {
        Ok(emojis) => emojis,
        Err(why) => {
            log::error!("Failed to get guild emojis: {}", why);
            return;
        }
    };

    // Get the cached channel
    let cache = match sqlx::query!(
        r#"SELECT * FROM ttc_emoji_cache_channels WHERE channel_id = $1"#,
        event.channel_id.0 as i64
    )
    .fetch_one(&data.pool)
    .await
    {
        Ok(cache) => cache,
        Err(why) => match why {
            sqlx::Error::RowNotFound => return,
            _ => {
                log::error!("Unable to get cached channel from DB: {}", why);
                return;
            }
        },
    };
    // Get the old message
    let msg = match sqlx::query!(
        r#"SELECT * FROM ttc_message_cache WHERE message_id = $1"#,
        event.id.0 as i64
    )
    .fetch_one(&data.pool)
    .await
    {
        Ok(msg) => msg,
        Err(why) => match why {
            sqlx::Error::RowNotFound => {
                log::warn!("Deleted message not found in database, emoji cache may be inaccurate.");
                return;
            }
            _ => {
                log::error!("Error getting message from database: {}", why);
                return;
            }
        },
    };

    let new = match new {
        Some(new) => new.clone(),
        None => match event.channel_id.message(ctx, event.id).await {
            Ok(new) => new,
            Err(why) => {
                log::error!("Failed to fetch message: {}", why);
                return;
            }
        },
    };

    if new.id.created_at().timestamp() < cache.timestamp_unix {
        // Store possible modifications to the users emojis
        let mut emoji_cache = EmojiCache::new(&data.pool);
        for emoji in &emoji_list {
            let emoji_pattern = format!("<:{}:", emoji.name);
            let new_contains = new.content.contains(&emoji_pattern);
            let old_contains = msg.content.as_ref().unwrap().contains(&emoji_pattern);

            if new_contains && !old_contains {
                match emoji_cache
                    .increase_emoji_count(new.author.id.0, emoji.name.clone(), 1)
                    .await
                {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("Failed to increase emoji counts in DB: {}", why);
                        return;
                    }
                }
            } else if !new_contains && old_contains {
                match emoji_cache
                    .decrease_emoji_count(new.author.id.0, emoji.name.clone(), 1)
                    .await
                {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("Failed to decrease emoji counts in DB: {}", why);
                        return;
                    }
                }
            }
        }
    }
}
