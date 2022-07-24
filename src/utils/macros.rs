/*#[macro_export]
macro_rules! get_config {
    ( $data:expr ) => {{
        let pool = &$data.pool;
        match crate::types::Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("error getting config from database: {}", why);
                return;
            }
        }
    }};
    ( $data:expr, $on_error:block ) => {{
        let pool = &$data.pool;
        match crate::types::Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("Error getting config from database: {}", why);
                $on_error;
            }
        }
    }};
}*/

#[macro_export]
macro_rules! command_error {
    ( $arg:expr ) => {
        Err(crate::Error::from($arg))
    };
    ( $fmt:expr, $( $arg:tt )* ) => {
        Err(crate::Error::from(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! config_function {
    ($sql:expr, $name:ident, Vec<$_type:ty>) => {
        pub async fn $name(&self) -> Result<Vec<$_type>, ::sqlx::Error> {
            Ok(::sqlx::query!($sql)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|record| record.$name)
                .collect::<Vec<$_type>>())
        }
    };

    ($sql:expr, $name:ident, $name_2:ident, Vec<$_type:ty>) => {
        pub async fn $name(&self) -> Result<Vec<$_type>, ::sqlx::Error> {
            Ok(::sqlx::query!($sql)
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|record| (record.$name, record.$name_2))
                .collect::<Vec<$_type>>())
        }
    };

    ($sql:expr, $name:ident, $_type:ty) => {
        pub async fn $name(&self) -> Result<$_type, ::sqlx::Error> {
            Ok(::sqlx::query!($sql).fetch_one(&self.pool).await?.$name)
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_return {
    ($_data:expr, $_str:expr) => {
        match $_data {
            Ok(data) => data,
            Err(why) => {
                ::log::error!("{}: {}", $_str, why);
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! embed_color {
    ($_name:ident, $_default_color:expr) => {
        pub async fn $_name(&self) -> ::poise::serenity_prelude::Color {
            let data = match sqlx::query!(
                r#"SELECT color FROM ttc_embed_colors WHERE embed_type = $1"#,
                stringify!($_name)
            )
            .fetch_one(&self.pool)
            .await
            {
                Ok(c) => Ok(c.color),
                Err(why) => Err(why),
            };
            match data {
                Ok(data) => {
                    if data.len() >= 3 {
                        return Color::from_rgb(data[0], data[1], data[2]);
                    } else {
                        ::log::warn!("Not enough color bytes in Database");
                        return $_default_color;
                    }
                }
                Err(why) => {
                    ::log::warn!(
                        "Error getting color \"{}\" for reply: {}",
                        stringify!($_name),
                        why
                    );
                    $_default_color
                }
            }
        }
    };
}

#[macro_export]
macro_rules! ttc_reply_generate {
    ($_fname:ident, $_defcolor:expr) => {
        pub async fn $_fname<T>(
            ctx: &'_ Context<'_, Data, Error>,
            title: T,
            description: T,
            ephemeral: bool,
        ) -> Result<(), Error>
        where
            T: ToString,
        {
            let color = ctx.data().$_fname().await;
            ttc_reply(ctx, color, ephemeral, title, description, vec![]).await?;
            Ok(())
        }
    };

    ($_fname:ident, $_defcolor:expr, $_ephemeral:expr) => {
        pub async fn $_fname<T>(
            ctx: &'_ Context<'_, Data, Error>,
            title: T,
            description: T,
        ) -> Result<(), Error>
        where
            T: ToString,
        {
            let color = ctx.data().$_fname().await;
            ttc_reply(ctx, color, $_ephemeral, title, description, vec![]).await?;
            Ok(())
        }
    };
}
