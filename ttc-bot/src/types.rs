use poise::serenity_prelude::UserId;
use sqlx::PgPool;

pub struct Data {
    pub users_currently_questioned: Vec<UserId>,
    pub pool: PgPool,
    pub thread_name_regex: regex::Regex,
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
}
