use sqlx::PgPool;
use std::sync::Arc;

/// A macro to generate functions to fetch config items
macro_rules! config_function {
    ($sql:expr, Vec<$_type:ty>, $name:ident) => {
        pub async fn $name(&self) -> Result<Vec<$_type>, ::sqlx::Error> {
            Ok(::sqlx::query!($sql)
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|record| record.$name)
                .collect::<Vec<$_type>>())
        }
    };

    ($sql:expr, Vec<$_type:ty>, $name:ident, $($additional_name:ident),+) => {
        pub async fn $name(&self) -> Result<Vec<$_type>, ::sqlx::Error> {
            Ok(::sqlx::query!($sql)
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|record| (record.$name, $(record.$additional_name,)+))
                .collect::<Vec<$_type>>())
        }
    };

    ($sql:expr, $_type:ty, $name:ident) => {
        pub async fn $name(&self) -> Result<$_type, ::sqlx::Error> {
            Ok(::sqlx::query!($sql).fetch_one(&*self.pool).await?.$name)
        }
    };
}

/// The struct to contain the functions to retrieve config keys
pub struct Config {
    pool: Arc<PgPool>,
}

impl Config {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.welcome_channel as welcome_channel
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        i64,
        welcome_channel
    );
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.verified_role as verified_role
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        i64,
        verified_role
    );
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.moderator_role as moderator_role
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        i64,
        moderator_role
    );
    config_function!(
        r#"select distinct
        tcbc.id as conveyance_blacklist_id,
        tcbc.channel_id as conveyance_blacklist_channel
        from ttc_config tc
        inner join ttc_conveyance_blacklist_channel tcbc on tc.conveyance_blacklist_id  = tcbc.id order by tcbc.id asc"#,
        Vec<i64>,
        conveyance_blacklist_channel
    );
    config_function!(
        r#"select distinct
        tcc.id as conveyance_id,
        tcc.channel_id as conveyance_channel
        from ttc_config tc
        inner join ttc_conveyance_channel tcc on tc.conveyance_id = tcc.id order by tcc.id asc"#,
        Vec<i64>,
        conveyance_channel
    );
    config_function!(
        r#"select distinct
        the.id as harold_emoji_id,
        the."name" as harold_emoji
        from ttc_config tc
        inner join ttc_harold_emoji the on tc.harold_emoji_id = the.id order by the.id asc"#,
        Vec<String>,
        harold_emoji
    );
    config_function!(
        r#"select role_id as selfroles, emoji_name
        from ttc_selfroles"#,
        Vec<(i64, Option<String>)>,
        selfroles,
        emoji_name
    );
}
