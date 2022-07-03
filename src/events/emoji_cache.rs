use poise::serenity_prelude::{ChannelId, Context, Message, MessageId, MessageUpdateEvent};

use crate::types::Data;

/// The event to account for message deletions in emoji caching
pub async fn message_delete(
    channel_id: &ChannelId,
    deleted_message_id: &MessageId,
    data: &Data,
) {
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
}
