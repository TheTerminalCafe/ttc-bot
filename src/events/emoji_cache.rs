use poise::serenity_prelude::{ChannelId, Context, Message, MessageId, MessageUpdateEvent};
use crate::{types::Data, utils::emoji_cache::EmojiCache};
use std::collections::HashMap;


/// The event to account for message deletions in emoji caching
pub async fn message_delete(
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
        let mut emoji_cache = match sqlx::query!(
            r#"SELECT * FROM ttc_emoji_cache WHERE user_id = $1"#,
            msg.user_id
        )
        .fetch_all(&data.pool)
        .await
        {
            Ok(emoji_cache) => emoji_cache,
            Err(why) => match why {
                sqlx::Error::RowNotFound => {
                    return;
                }
                _ => {
                    log::error!("Error getting message from database: {}", why);
                    return;
                }
            },
        };
        // Avoid updating all the entries if it is not needed
        let mut emojies_changed = false;
        // Loop through all the records and decrease counts accordingly
        for record in &mut emoji_cache {
            if msg
                .content
                .as_ref()
                .unwrap()
                .contains(&format!("<:{}:", record.emoji_name))
            {
                emojies_changed = true;
                record.emoji_count -= 1;
            }
        }
        if emojies_changed {
            for record in &emoji_cache {
                match sqlx::query!(r#"UPDATE ttc_emoji_cache SET emoji_count = $1 WHERE emoji_name = $2 AND user_id = $3"#, 
                    record.emoji_count, 
                    record.emoji_name, 
                    record.user_id)
                .execute(&data.pool)
                .await {
                    Ok(_) => (), 
                    Err(why) => { 
                        log::error!("Failed to update emoji counts: {}", why) 
                    }
                }
            }
        }
        match sqlx::query!(
            r#"UPDATE ttc_emoji_cache_messages 
            SET num_messages = num_messages - 1 
            WHERE user_id = $1"#,
            msg.user_id).execute(&data.pool).await {
            Ok(_) => (),
            Err(why) => {
                log::error!("Error updating message counts: {}", why)
            }
        }
        match sqlx::query!(r#"
            UPDATE ttc_emoji_cache_messages 
            SET num_messages = num_messages - 1 
            WHERE user_id = 0"#).execute(&data.pool).await {
            Ok(_) => (),
            Err(why) => {
                log::error!("Error updating message counts: {}", why)
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
        None => return 
    }.emojis(ctx).await { 
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
        }
    };
    
    if new.id.created_at().timestamp() < cache.timestamp_unix {
        // Store possible modifications to the users emojis
        let mut map = HashMap::new();
        for emoji in &emoji_list {
            let emoji_pattern = format!("<:{}:", emoji.name);
            let new_contains = new.content.contains(&emoji_pattern);
            let old_contains = msg.content.as_ref().unwrap().contains(&emoji_pattern);
            
            if new_contains && !old_contains {
                map.insert(emoji.name.clone(), false);
            } else if !new_contains && old_contains {
                map.insert(emoji.name.clone(), true);
            }
        }
        
        for (name, increment) in map {
            if increment {
                match sqlx::query!(r#"SELECT * FROM ttc_emoji_cache WHERE user_id = $1 AND emoji_name = $2"#, msg.user_id, name).fetch_one(&data.pool).await {
                    Ok(_) => {
                        match sqlx::query!(r#"UPDATE ttc_emoji_cache SET emoji_count = emoji_count + 1 WHERE user_id = $1 AND emoji_name = $2"#, msg.user_id, name).execute(&data.pool).await {
                            Ok(_) => (),
                            Err(why) => {
                                log::error!("Failed to update emoji counts in DB: {}", why);
                                return;
                            }
                        }
                    }
                    Err(why) => {
                        match why {
                            sqlx::Error::RowNotFound => {
                                match sqlx::query!(r#"INSERT INTO ttc_emoji_cache VALUES($1, $2, 1)"#, msg.user_id, name).execute(&data.pool).await {
                                    Ok(_) => (),
                                    Err(why) => {
                                        log::error!("Failed to update emoji counts in DB: {}", why);
                                        return;
                                    }
                                }
                                
                            }
                            _ => {
                                log::error!("Failed to update emoji counts in DB: {}", why);
                                return;
                            }
                        }
                    }
                }
            } else {
                match sqlx::query!(r#"UPDATE ttc_emoji_cache SET emoji_count = emoji_count - 1 WHERE user_id = $1 AND emoji_name = $2"#, msg.user_id, name).execute(&data.pool).await {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("Failed to update emoji counts in DB: {}", why);
                        return;
                    }
                }            
            }
        }
    }
}
