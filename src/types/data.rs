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
