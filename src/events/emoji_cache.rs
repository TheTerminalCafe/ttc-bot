use poise::serenity_prelude::{ChannelId, Context, Message, MessageId, MessageUpdateEvent};

use crate::types::Data;

pub async fn message_delete(
    ctx: &Context,
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
}

pub async fn message_update(
    ctx: &Context,
    new: &Option<Message>,
    event: &MessageUpdateEvent,
    data: &Data,
) {
}
