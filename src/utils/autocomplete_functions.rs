use futures::{Stream, StreamExt};

use crate::Context;

pub async fn language_autocomplete<'a>(
    _: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    futures::stream::iter(crate::commands::localisation::LANGUAGE_CODES)
        .filter(move |code| {
            futures::future::ready(code.1.to_lowercase().starts_with(&partial.to_lowercase()))
        })
        .map(|code| code.1.to_string())
}
