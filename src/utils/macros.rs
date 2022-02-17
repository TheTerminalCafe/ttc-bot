#[macro_export]
macro_rules! get_config {
    ( $ctx:expr ) => {{
        let data = $ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match crate::typemap::config::Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("error getting config from database: {}", why);
                return;
            }
        }
    }};
    ( $ctx:expr, $on_error:block ) => {{
        let data = $ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match crate::typemap::config::Config::get_from_db(pool).await {
            Ok(config) => config,
            Err(why) => {
                log::error!("Error getting config from database: {}", why);
                $on_error;
            }
        }
    }};
}

#[macro_export]
macro_rules! command_error {
    ( $expr:expr ) => {
        Err(serenity::framework::standard::CommandError::from($expr))
    };
}
