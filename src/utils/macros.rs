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
    ($sql:expr, $name:ident, $_type:ty) => {
        pub async fn $name(&self) -> Result<$_type, ::sqlx::Error> {
            Ok(::sqlx::query!($sql).fetch_one(&self.pool).await?.$name)
        }
    };
}
