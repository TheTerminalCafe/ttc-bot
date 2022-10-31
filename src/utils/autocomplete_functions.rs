use futures::{Stream, StreamExt};

use crate::Context;

pub async fn language_autocomplete(_: Context<'_>, partial: String) -> impl Stream<Item = String> {
    futures::stream::iter(crate::commands::localisation::LANGUAGE_CODES)
        .filter(move |code| {
            futures::future::ready(code.1.to_lowercase().starts_with(&partial.to_lowercase()))
        })
        .map(|code| code.1.to_string())
}
