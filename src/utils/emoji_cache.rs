use futures::StreamExt;
use poise::serenity_prelude::{Context, GuildId};
use sqlx::{Pool, Postgres};

use crate::types::{self, Error};
use std::{collections::HashMap, sync::atomic::AtomicBool};

// idk how to put this in the struct as shared static
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug)]
pub struct CacheData {
    /// HashMap<emoji, HashMap<userid, count>>
    user_emojis: HashMap<(u64, String), u64>,
    /// HashMap<userid, count>
    user_messages: HashMap<u64, u64>,
}

impl CacheData {
    pub fn new() -> Self {
        Self {
            user_emojis: HashMap::new(),
            user_messages: HashMap::new(),
        }
    }

    pub fn increase_user_emojis(&mut self, uid: u64, emoji: String, count: u64) {
        *self.user_emojis.entry((uid, emoji)).or_insert(0) += count;
    }

    pub fn increase_user_messages(&mut self, uid: u64, count: u64) {
        *self.user_messages.entry(uid).or_insert(0) += count;
    }

    pub fn decrease_emoji_count(
        &mut self,
        uid: u64,
        emoji: String,
        count: u64,
    ) -> Result<(), Error> {
        if !self.user_emojis.contains_key(&(uid, emoji.clone())) {
            return Err(Error::from("User has no count for this Emoji"));
        }
        *self.user_emojis.get_mut(&(uid, emoji)).unwrap() -= count;
        Ok(())
    }

    pub fn decrease_message_count(&mut self, uid: u64, count: u64) -> Result<(), Error> {
        if !self.user_messages.contains_key(&uid) {
            return Err(Error::from("User has no message count"));
        }
        *self.user_messages.get_mut(&uid).unwrap() -= count;
        Ok(())
    }

    pub fn filter(&mut self, uids: Vec<u64>, emojis: Vec<String>) {
        self.user_messages
            .retain(|k, _| uids.contains(k) || *k == 0);
        self.user_emojis
            .retain(|k, _| (uids.contains(&k.0) || k.0 == 0) && emojis.contains(&k.1));
    }

    pub fn user_emojis_vec(&self) -> Vec<(u64, String, u64)> {
        let mut res: Vec<(u64, String, u64)> = Vec::new();
        for (k, v) in &self.user_emojis {
            res.push((k.0, k.1.clone(), *v));
        }
        return res;
    }

    pub fn user_message_vec(&self) -> Vec<(u64, u64)> {
        let mut res: Vec<(u64, u64)> = Vec::new();
        for (k, v) in &self.user_messages {
            res.push((*k, *v));
        }
        return res;
    }

    pub fn user_emojis_hash_emoji_user(&self) -> HashMap<String, HashMap<u64, u64>> {
        let mut res: HashMap<String, HashMap<u64, u64>> = HashMap::new();
        for (k, v) in &self.user_emojis {
            res.entry(k.1.clone())
                .or_insert(HashMap::new())
                .insert(k.0, *v);
        }
        return res;
    }

    pub fn user_messages(&self) -> HashMap<u64, u64> {
        self.user_messages.clone()
    }
}

pub struct EmojiCache<'a> {
    pool: &'a Pool<Postgres>,
    cached_data: Option<CacheData>,
}

impl<'a> EmojiCache<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        Self {
            pool,
            cached_data: None,
        }
    }

    /// Get the cache from the itself cache or the DB
    pub async fn get_data(&mut self) -> Result<CacheData, Error> {
        if IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::from("The emoji cache is currently being updated"));
        }
        if let None = self.cached_data {
            self.get_database_data().await?;
        }
        // Due to the get_database_data it can't be None
        Ok(self.cached_data.as_ref().unwrap().clone())
    }

    /// Get the current cache from the Database without updating it first
    ///
    /// You should check ``is_running`` first since you will get an Error otherwise
    async fn get_database_data(&mut self) -> Result<(), Error> {
        if IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::from("The emoji cache is currently being updated"));
        }

        let mut cr = CacheData::new();
        for row in sqlx::query!(r#"SELECT * FROM ttc_emoji_cache"#)
            .fetch_all(self.pool)
            .await?
        {
            cr.increase_user_emojis(row.user_id as u64, row.emoji_name, row.emoji_count as u64);
        }

        for row in sqlx::query!(r#"SELECT * FROM ttc_emoji_cache_messages"#)
            .fetch_all(self.pool)
            .await?
        {
            cr.increase_user_messages(row.user_id as u64, row.num_messages as u64);
        }
        self.cached_data = Some(cr.clone());
        Ok(())
    }

    /// Decreases the Emoji count
    pub async fn decrease_emoji_count(
        &mut self,
        user_id: u64,
        emoji: String,
        count: u64,
    ) -> Result<(), Error> {
        if let Some(data) = &mut self.cached_data {
            data.decrease_emoji_count(user_id, emoji.clone(), count)?;
            data.decrease_emoji_count(0, emoji.clone(), count)?;
        }
        let user_id = user_id as i64;
        let count = count as i64;
        sqlx::query!(
            r#"UPDATE ttc_emoji_cache SET emoji_count = emoji_count - $3 WHERE user_id = $1 AND emoji_name = $2"#,
            user_id,
            emoji,
            count
        )
        .execute(self.pool)
        .await?;
        sqlx::query!(
            r#"UPDATE ttc_emoji_cache SET emoji_count = emoji_count - $2 WHERE user_id = 0 AND emoji_name = $1"#,
            emoji,
            count
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Decreases only the Message count
    pub async fn decrease_message_count(&mut self, user_id: u64, count: u64) -> Result<(), Error> {
        if let Some(data) = &mut self.cached_data {
            data.decrease_message_count(user_id, count)?;
            data.decrease_message_count(0, count)?;
        }
        let user_id = user_id as i64;
        let count = count as i64;
        sqlx::query!(
            r#"UPDATE ttc_emoji_cache_messages SET num_messages = num_messages - $2 WHERE user_id = $1"#,
            user_id,
            count
        )
        .execute(self.pool)
        .await?;
        sqlx::query!(
            r#"UPDATE ttc_emoji_cache_messages SET num_messages = num_messages - $1 WHERE user_id = 0"#,
            count
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Increases the Emoji count
    pub async fn increase_emoji_count(
        &mut self,
        user_id: u64,
        emoji: String,
        count: u64,
    ) -> Result<(), Error> {
        if let Some(data) = &mut self.cached_data {
            data.increase_user_emojis(user_id, emoji.clone(), count);
            data.increase_user_emojis(0, emoji.clone(), count);
        }
        let user_id = user_id as i64;
        let count = count as i64;
        sqlx::query!(
            r#"
            INSERT INTO ttc_emoji_cache VALUES($1, $2, $3) 
            ON CONFLICT (user_id, emoji_name) DO UPDATE SET emoji_count = ttc_emoji_cache.emoji_count + $3
            "#,
            user_id,
            emoji,
            count
        )
        .execute(self.pool)
        .await?;
        sqlx::query!(
            r#"
            INSERT INTO ttc_emoji_cache VALUES(0, $1, $2) 
            ON CONFLICT (user_id, emoji_name) DO UPDATE SET emoji_count = ttc_emoji_cache.emoji_count + $2
            "#,
            emoji,
            count
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub fn is_running() -> bool {
        IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Update the Emoji Cache and then return the result
    ///
    /// You should call ``is_running`` before to ensure it isn't running. Otherwise you will get an
    /// Error. The argument ``full_rebuild`` specifies, if **every** message should be rescanned or
    /// if it should continue from the last known point.
    /// Please note that the UserID 0 is used for global messages
    pub async fn update_emoji_cache_poise(
        &mut self,
        ctx: &'a types::Context<'_>,
        full_rebuild: bool,
    ) -> Result<(), Error> {
        let guild = match ctx.guild_id() {
            Some(guild_id) => guild_id,
            None => {
                return Err(Error::from(
                    "The poise Context did not contain a valid guild id.",
                ))
            }
        };
        self.update_emoji_cache(ctx.discord(), guild, full_rebuild)
            .await
    }

    /// Update the Emoji Cache and then return the result
    ///
    /// You should call ``is_running`` before to ensure it isn't running. Otherwise you will get an
    /// Error. The argument ``full_rebuild`` specifies, if **every** message should be rescanned or
    /// if it should continue from the last known point.
    /// Please note that the UserID 0 is used for global messages
    pub async fn update_emoji_cache(
        &mut self,
        ctx: &'a Context,
        guild: GuildId,
        full_rebuild: bool,
    ) -> Result<(), Error> {
        if full_rebuild {
            let cr = CacheData::new();
            self.inner_update_emoji_cache(ctx, guild, cr, HashMap::new())
                .await
        } else {
            let data = self.get_data().await?;
            let mut channel_progress: HashMap<u64, (u64, i64)> = HashMap::new();
            let channel_progress_raw = sqlx::query!(r#"SELECT * FROM ttc_emoji_cache_channels"#)
                .fetch_all(self.pool)
                .await?;
            for row in channel_progress_raw {
                channel_progress.insert(
                    row.channel_id as u64,
                    (row.message_id as u64, row.timestamp_unix as i64),
                );
            }
            self.inner_update_emoji_cache(ctx, guild, data, channel_progress)
                .await
        }
    }

    async fn inner_update_emoji_cache(
        &mut self,
        ctx: &'a Context,
        guild: GuildId,
        mut data: CacheData,
        channel_progress: HashMap<u64, (u64, i64)>,
    ) -> Result<(), Error> {
        if IS_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::from("The emoji cache is already being updated"));
        }
        IS_RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);

        let mut handles = Vec::new();
        let emoji_names: Vec<String> = guild
            .emojis(ctx)
            .await?
            .into_iter()
            .map(|e| (e.name))
            .collect();

        for (channel_id, _) in guild.channels(ctx).await? {
            let ctx = ctx.clone();
            let emoji_names = emoji_names.clone();
            let last_message_in_cache = channel_progress
                .get(&channel_id.0)
                .unwrap_or(&(0, 0))
                .clone();
            let handle = tokio::spawn(async move {
                let mut messages = channel_id.messages_iter(ctx).boxed();
                let mut user_emoji_entries: HashMap<(u64, String), u64> = HashMap::new();
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
                                    *user_emoji_entries
                                        .entry((0, emoji.to_string()))
                                        .or_insert(0) += 1;
                                    *user_emoji_entries
                                        .entry((message.author.id.0, emoji.to_string()))
                                        .or_insert(0) += 1;
                                }
                            }
                        }
                        Err(why) => log::error!("error getting message for emoji cache: {}", why),
                    }
                }
                (user_emoji_entries, newest_message, user_msg_count)
            });
            handles.push(handle);
        }

        // Tuple magic...
        let mut channel_progress = Vec::new();
        for handle in handles {
            let (user_emojis, newest_message, message_counts) = handle.await?;
            for (k, v) in user_emojis {
                data.increase_user_emojis(k.0, k.1, v);
            }
            for (k, v) in message_counts {
                data.increase_user_messages(k, v);
            }
            channel_progress.push((
                newest_message.0 as i64,
                newest_message.1 as i64,
                newest_message.2,
            ));
        }

        // -----------------------
        // Filtering out old stuff
        // -----------------------
        let mut server_users = Vec::new();
        let mut members = guild.members_iter(ctx).boxed();
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

        // Remove old users + emojis
        data.filter(server_users, emoji_names);

        // Remove old channels
        let server_channels = guild
            .channels(ctx)
            .await?
            .into_iter()
            .map(|c| (c.0 .0))
            .collect::<Vec<u64>>();

        let channel_progress = channel_progress
            .into_iter()
            .filter(|c| (server_channels.contains(&(c.0 as u64))))
            .collect::<Vec<(i64, i64, i64)>>();

        // Re-insert the Data in the DB
        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache"#)
            .execute(self.pool)
            .await?;
        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache_messages"#)
            .execute(self.pool)
            .await?;
        sqlx::query!(r#"TRUNCATE TABLE ttc_emoji_cache_channels"#)
            .execute(self.pool)
            .await?;

        for channel in channel_progress {
            sqlx::query!(
            r#"INSERT INTO ttc_emoji_cache_channels (channel_id, message_id, timestamp_unix) VALUES ($1, $2, $3)"#,
            channel.0,
            channel.1,
            channel.2
            )
            .execute(self.pool)
            .await?;
        }

        for (user, emoji, count) in data.user_emojis_vec() {
            sqlx::query!(
                    r#"INSERT INTO ttc_emoji_cache (user_id, emoji_name, emoji_count) VALUES ($1, $2, $3)"#,
                    user as i64,
                    emoji,
                    count as i64
            )
            .execute(self.pool)
            .await?;
        }

        for (user, vcount) in data.user_message_vec() {
            sqlx::query!(
                r#"INSERT INTO ttc_emoji_cache_messages (user_id, num_messages) VALUES ($1, $2)"#,
                user as i64,
                vcount as i64
            )
            .execute(self.pool)
            .await?;
        }

        IS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
