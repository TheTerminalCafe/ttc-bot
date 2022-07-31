use std::{collections::HashMap, sync::Arc, time::Instant};

use poise::serenity_prelude::{ChannelId, Message, RwLock, UserId, Webhook};
use sqlx::PgPool;

use crate::{
    types::{colors::Colors, config::Config},
    utils::bee_utils::{BeeifiedUser, BeezoneChannel},
};

pub struct Data {
    pub users_currently_questioned: RwLock<Vec<UserId>>,
    pub harold_message: RwLock<Option<Message>>,
    pub beeified_users: RwLock<HashMap<UserId, BeeifiedUser>>,
    pub beezone_channels: RwLock<HashMap<ChannelId, BeezoneChannel>>,
    pub webhooks: RwLock<HashMap<ChannelId, Webhook>>,
    pub pool: Arc<PgPool>,
    pub thread_name_regex: regex::Regex,
    pub startup_time: Instant,
    pub config: Config,
    pub colors: Colors,
}

/*
/// Implementations of the config keys in the database using the macro to reduce code duplication
*/

/*#[derive(Debug, Clone)]
pub struct Config {
    pub support_channel: i64,
    pub conveyance_channels: Vec<i64>,
    pub conveyance_blacklisted_channels: Vec<i64>,
    pub welcome_channel: i64,
    pub verified_role: i64,
    pub moderator_role: i64,
    pub welcome_messages: Vec<String>,
}

impl Config {
    pub async fn save_in_db(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM ttc_config"#)
            .execute(pool)
            .await?;

        sqlx::query!(
            r#"INSERT INTO ttc_config VALUES($1, $2, $3, $4, $5, $6, $7)"#,
            self.support_channel,
            &self.conveyance_channels,
            &self.conveyance_blacklisted_channels,
            self.welcome_channel,
            self.verified_role,
            self.moderator_role,
            &self.welcome_messages,
        )
        .execute(pool)
        .await?;

        log::info!("Settings saved.");

        Ok(())
    }

    pub async fn get_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(Self, r#"SELECT * FROM ttc_config"#)
            .fetch_one(pool)
            .await
    }
}*/
