#[macro_export]
macro_rules! command_error {
    ( $arg:expr ) => {
        Err($crate::Error::from($arg))
    };
    ( $fmt:expr, $( $arg:tt )* ) => {
        Err($crate::Error::from(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! command_raw_error {
    ( $arg:expr ) => {
        $crate::Error::from($arg)
    };
    ( $fmt:expr, $( $arg:tt )* ) => {
        $crate::Error::from(format!($fmt, $($arg)*))
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
