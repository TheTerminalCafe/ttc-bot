use std::{collections::HashMap, time::Instant};

use poise::serenity_prelude::{ChannelId, Message, RwLock, UserId, Webhook};
use sqlx::PgPool;

use crate::utils::bee_utils::{BeeifiedUser, BeezoneChannel};

pub struct Data {
    pub users_currently_questioned: RwLock<Vec<UserId>>,
    pub harold_message: RwLock<Option<Message>>,
    pub beeified_users: RwLock<HashMap<UserId, BeeifiedUser>>,
    pub beezone_channels: RwLock<HashMap<ChannelId, BeezoneChannel>>,
    pub webhooks: RwLock<HashMap<ChannelId, Webhook>>,
    pub pool: PgPool,
    pub thread_name_regex: regex::Regex,
    pub startup_time: Instant,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, Clone)]
pub struct Config {
    pub support_channel: i64,
    pub conveyance_channels: Vec<i64>,
    pub conveyance_blacklisted_channels: Vec<i64>,
    pub welcome_channel: i64,
    pub verified_role: i64,
    pub moderator_role: i64,
    pub welcome_messages: Vec<String>,
    pub harold_emojis: Vec<String>,
}

impl Config {
    pub async fn save_in_db(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM ttc_config"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"DELETE FROM ttc_config_properties"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"DELETE FROM ttc_conveyance_blacklist_channel"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"DELETE FROM ttc_conveyance_channel"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"DELETE FROM ttc_harold_emoji"#)
            .execute(pool)
            .await?;
        sqlx::query!(r#"DELETE FROM ttc_welcome_message"#)
            .execute(pool)
            .await?;
        let mut ttc_conveyance_channel_ids = Vec::new();
        for id in &self.conveyance_channels {
            ttc_conveyance_channel_ids.push(
                sqlx::query!(
                    r#"INSERT INTO ttc_conveyance_channel (channel_id) VALUES ($1) RETURNING id"#,
                    id
                )
                .fetch_one(pool)
                .await?
                .id,
            );
        }
        let mut ttc_conveyance_blacklist_channel_ids = Vec::new();
        for id in &self.conveyance_blacklisted_channels {
            ttc_conveyance_blacklist_channel_ids.push(
                sqlx::query!(
                    r#"INSERT INTO ttc_conveyance_blacklist_channel (channel_id) VALUES ($1) RETURNING id"#,
                    id
                ).fetch_one(pool).await?.id
            );
        }
        let ttc_config_properties = sqlx::query!(r#"
            INSERT INTO ttc_config_properties (support_channel, welcome_channel, verified_role, moderator_role)
            VALUES ($1, $2, $3, $4) RETURNING id
            "#,
            self.support_channel,
            self.welcome_channel,
            self.verified_role,
            self.moderator_role
            ).fetch_one(pool).await?.id;
        let mut ttc_harold_emoji = Vec::new();
        for name in &self.harold_emojis {
            ttc_harold_emoji.push(
                sqlx::query!(
                    r#"INSERT INTO ttc_harold_emoji (name) VALUES ($1) RETURNING id"#,
                    name
                )
                .fetch_one(pool)
                .await?
                .id,
            );
        }
        let mut ttc_welcome_message = Vec::new();
        for msg in &self.welcome_messages {
            ttc_welcome_message.push(
                sqlx::query!(
                    r#"INSERT INTO ttc_welcome_message (welcome_message) VALUES ($1) RETURNING id"#,
                    msg
                )
                .fetch_one(pool)
                .await?
                .id,
            );
        }

        let mut cnt = 0;
        loop {
            let conv_id = ttc_conveyance_channel_ids.get(cnt);
            let conv_bl_id = ttc_conveyance_blacklist_channel_ids.get(cnt);
            let welc = ttc_welcome_message.get(cnt);
            let har = ttc_harold_emoji.get(cnt);
            if conv_id.is_none() && conv_bl_id.is_none() && welc.is_none() && har.is_none() {
                break;
            }
            sqlx::query!(
                r#"
                INSERT INTO ttc_config (config_properties_id, conveyance_id, conveyance_blacklist_id, welcome_message_id, harold_emoji_id)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                ttc_config_properties,
                conv_id,
                conv_bl_id,
                welc,
                har,
            ).execute(pool).await?;
            cnt += 1;
        }

        log::info!("Settings saved.");

        Ok(())
    }

    pub async fn get_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let raw = sqlx::query!(
            r#"
            select
            tc.id as config_id,
            tcp.id as config_properties_id,
            tcp.support_channel as supprt_channel,
            tcp.welcome_channel as welcome_channel,
            tcp.verified_role as verified_role,
            tcp.moderator_role as moderator_role,
            tcbc.channel_id as conveyance_blacklist_channel, 
            tcc.channel_id as conveyance_channel, 
            the."name" as harold_emoji, 
            twm.welcome_message as welcome_message 
            from ttc_config tc
            full join ttc_config_properties tcp on tc.config_properties_id = tcp.id
            full join ttc_conveyance_blacklist_channel tcbc on tc.conveyance_blacklist_id = tcbc.id 
            full join ttc_conveyance_channel tcc on tc.conveyance_id = tcc.id 
            full join ttc_harold_emoji the on tc.harold_emoji_id = the.id 
            full join ttc_welcome_message twm on tc.welcome_message_id = twm.id;
            "#
        )
        .fetch_all(pool)
        .await?;
        // The unwrap is safe since every row contains config_properties_id
        let mut res = Self {
            conveyance_channels: Vec::new(),
            verified_role: raw.get(0).unwrap().verified_role.unwrap(),
            moderator_role: raw.get(0).unwrap().moderator_role.unwrap(),
            support_channel: raw.get(0).unwrap().supprt_channel.unwrap(),
            welcome_channel: raw.get(0).unwrap().welcome_channel.unwrap(),
            welcome_messages: Vec::new(),
            conveyance_blacklisted_channels: Vec::new(),
            harold_emojis: Vec::new(),
        };
        res.conveyance_channels.append(
            &mut raw
                .iter()
                .filter_map(|val| val.conveyance_channel)
                .collect::<Vec<i64>>(),
        );
        res.conveyance_blacklisted_channels.append(
            &mut raw
                .iter()
                .filter_map(|val| val.conveyance_blacklist_channel)
                .collect::<Vec<i64>>(),
        );
        res.welcome_messages.append(
            &mut raw
                .iter()
                .filter_map(|val| val.welcome_message.clone())
                .collect::<Vec<String>>(),
        );
        res.harold_emojis.append(
            &mut raw
                .iter()
                .filter_map(|val| val.harold_emoji.clone())
                .collect::<Vec<String>>(),
        );
        return Ok(res);
    }
}
