use futures::{Stream, StreamExt};
use poise::serenity_prelude::Member;

use crate::types::Context;

pub async fn language_autocomplete(_: Context<'_>, partial: String) -> impl Stream<Item = String> {
    futures::stream::iter(crate::commands::localisation::LANGUAGE_CODES)
        .filter(move |code| futures::future::ready(code.1.starts_with(&partial)))
        .map(|code| code.1.to_string())
}