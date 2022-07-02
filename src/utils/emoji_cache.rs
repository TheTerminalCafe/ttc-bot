use futures::StreamExt;
use poise::{
    serenity_prelude::{Context, Guild},
    Context as ContextPoise,
};
use sqlx::{Pool, Postgres};

use crate::types::{Data, Error};
use std::{collections::HashMap, sync::atomic::AtomicBool};

// idk how to put this in the struct as shared static
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

pub struct CacheData {
    /// HashMap<(userid, emoji), count>
    pub user_emojis: HashMap<(u64, String), u64>,
    /// HashMap<(userid, count)>
    pub user_messages: HashMap<u64, u64>,
}

pub struct EmojiCache {
    pool: Pool<Postgres>,
    ctx_serenity: Context,
    guild: Guild,
}

impl<'a> EmojiCache {
    pub fn new(ctx: &'a ContextPoise<'_, Data, Error>) -> Self {
        Self {
            pool: ctx.data().pool.clone(),
            ctx_serenity: ctx.discord().clone(),
            guild: ctx.guild().unwrap().clone(),
        }
    }

    pub fn is_running(&self) -> bool {
        IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Update the Emoji Cache and then return the result
    ///
    /// You should call ``is_running`` before to ensure it isn't running. Otherwise you will get an
    /// Error. The argument ``full_rebuild`` specifies, if **every** message should be rescanned or
    /// if it should continue from the last known point.
    /// Please note that the UserID 0 is used for global messages
    pub async fn update_emoji_cache(&self, full_rebuild: bool) -> Result<CacheData, Error> {
        if IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::from("The emoji cache is already being updated"));
        }
        IS_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);

        let pool = &self.pool;
        let mut data: HashMap<(u64, String), u64> = HashMap::new();
        let mut message_count: HashMap<u64, u64> = HashMap::new();
        let mut channel_progress: HashMap<u64, (u64, i64)> = HashMap::new();

        // Get old counts from DB, if not building from scratch
        if !full_rebuild {
            let data_raw = sqlx::query!(r#"SELECT * FROM ttc_emoji_cache"#,)
                .fetch_all(pool)
                .await?;
            for row in data_raw {
                data.insert((row.user_id as u64, row.emoji_name), row.emoji_count as u64);
            }
            let channel_progress_raw = sqlx::query!(r#"SELECT * FROM ttc_emoji_cache_channels"#)
                .fetch_all(pool)
                .await?;
            for row in channel_progress_raw {
                channel_progress.insert(
                    row.channel_id as u64,
                    (row.message_id as u64, row.timestamp_unix as i64),
                );
            }
            let message_count_raw = sqlx::query!(r#"SELECT * FROM ttc_emoji_cache_messages"#)
                .fetch_all(pool)
                .await?;
            for row in message_count_raw {
                message_count.insert(row.user_id as u64, row.num_messages as u64);
            }
        }

        let mut handles = Vec::new();
        let emoji_names: Vec<String> = self
            .guild
            .emojis(self.ctx_serenity.clone())
            .await?
            .into_iter()
            .map(|e| (e.name))
            .collect();

        for (channel_id, _) in self.guild.channels.clone() {
            let ctx = self.ctx_serenity.clone();
            let emoji_names = emoji_names.clone();
            let last_message_in_cache = channel_progress
                .get(&channel_id.0)
                .unwrap_or(&(0, 0))
                .clone();
            let handle = tokio::spawn(async move {
                let mut messages = channel_id.messages_iter(ctx).boxed();
                let mut user_emoji_entires: HashMap<(u64, String), u64> = HashMap::new();
                let mut user_msg_count: HashMap<u64, u64> = HashMap::new();
                let mut newest_message = (channel_id.0, 0, 0);
                while let Some(message) = messages.next().await {
                    match message {
                        Ok(message) => {
                            // Dirty hack to run this once since the first message is the newest
                            if newest_message.1 == 0 {
                                newest_message = (
                                    channel_id.0,
                                    message.id.0,
                                    message.timestamp.unix_timestamp(),
                                );
                            }
                            // When we are at the value from last time
                            if (message.id.0 == last_message_in_cache.0)
                                || (message.timestamp.unix_timestamp() < last_message_in_cache.1)
                            {
                                break;
                            }
                            if message.author.bot {
                                continue;
                            }
                            *user_msg_count.entry(0).or_insert(0) += 1;
                            *user_msg_count.entry(message.author.id.0).or_insert(0) += 1;
                            for emoji in &emoji_names {
                                if message
                                    .content
                                    .contains(&format!("<:{}:", emoji).to_string())
                                {
                                    *user_emoji_entires
                                        .entry((0, emoji.to_string()))
                                        .or_insert(0) += 1;
                                    *user_emoji_entires
                                        .entry((message.author.id.0, emoji.to_string()))
                                        .or_insert(0) += 1;
                                }
                            }
                        }
                        Err(why) => log::error!("error getting message for emoji cache: {}", why),
                    }
                }
                (user_emoji_entires, newest_message, user_msg_count)
            });
            handles.push(handle);
        }

        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache_messages"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache_channels"#)
            .execute(pool)
            .await?;

        // Tuple magic...
        let mut channel_progress = Vec::new();
        for handle in handles {
            let value = handle.await?;
            for (k, v) in value.0 {
                *data.entry(k).or_insert(0) += v;
            }
            for (k, v) in value.2 {
                *message_count.entry(k).or_insert(0) += v;
            }
            channel_progress.push((value.1 .0 as i64, value.1 .1 as i64, value.1 .2));
        }

        // -----------------------
        // Filtering out old stuff
        // -----------------------

        let mut server_users = Vec::new();
        let mut members = self
            .guild
            .id
            .members_iter(self.ctx_serenity.clone())
            .boxed();
        while let Some(member) = members.next().await {
            match member {
                Ok(member) => {
                    server_users.push(member.user.id.0);
                }
                Err(why) => {
                    log::error!("error getting member: {}", why);
                }
            }
        }

        // Remove old users
        let mut new_data = HashMap::new();
        for row in data {
            if (server_users.contains(&row.0 .0) || row.0 .0 == 0)
                && emoji_names.contains(&row.0 .1)
            {
                new_data.insert((row.0 .0, row.0 .1), row.1);
            }
        }
        let data = new_data;
        let message_count = message_count
            .into_iter()
            .filter(|row| (server_users.contains(&row.0) || row.0 == 0))
            .collect::<HashMap<u64, u64>>();

        // Remove old channels
        let server_channels = self
            .guild
            .channels(self.ctx_serenity.clone())
            .await?
            .into_iter()
            .map(|c| (c.0 .0))
            .collect::<Vec<u64>>();

        let channel_progress = channel_progress
            .into_iter()
            .filter(|c| (server_channels.contains(&(c.0 as u64))))
            .collect::<Vec<(i64, i64, i64)>>();

        // Insert the Data in the DB
        for channel in channel_progress {
            sqlx::query!(
            r#"INSERT INTO ttc_emoji_cache_channels (channel_id, message_id, timestamp_unix) VALUES ($1, $2, $3)"#,
            channel.0,
            channel.1,
            channel.2
            )
            .execute(pool)
            .await?;
        }

        for (k, v) in &data {
            sqlx::query!(
            r#"INSERT INTO ttc_emoji_cache (user_id, emoji_name, emoji_count) VALUES ($1, $2, $3)"#,
            k.0 as i64,
            k.1,
            *v as i64
        )
        .execute(pool)
        .await?;
        }

        for (k, v) in &message_count {
            sqlx::query!(
                r#"INSERT INTO ttc_emoji_cache_messages (user_id, num_messages) VALUES ($1, $2)"#,
                *k as i64,
                *v as i64
            )
            .execute(pool)
            .await?;
        }

        IS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(CacheData {
            user_emojis: data,
            user_messages: message_count,
        })
    }
}
