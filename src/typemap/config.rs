use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct Config {
    pub support_channel: i64,
    pub conveyance_channel: i64,
    pub conveyance_blacklisted_channels: Vec<i64>,
    pub welcome_channel: i64,
    pub welcome_messages: Vec<String>,
}

impl Config {
    pub async fn save_in_db(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM ttc_config"#)
            .execute(pool)
            .await?;

        sqlx::query!(
            r#"INSERT INTO ttc_config VALUES($1, $2, $3, $4, $5)"#,
            self.support_channel,
            self.conveyance_channel,
            &self.conveyance_blacklisted_channels,
            self.welcome_channel,
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
