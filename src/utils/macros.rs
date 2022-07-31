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

/*
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
*/
