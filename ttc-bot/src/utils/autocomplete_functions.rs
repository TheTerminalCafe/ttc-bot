use futures::{Stream, StreamExt};
use poise::serenity_prelude::Member;

use crate::types::Context;

pub async fn language_autocomplete(_: Context<'_>, partial: String) -> impl Stream<Item = String> {
    futures::stream::iter(crate::commands::localisation::LANGUAGE_CODES)
        .filter(move |code| futures::future::ready(code.1.starts_with(&partial)))
        .map(|code| code.1.to_string())
}

pub async fn member_autocomplete(ctx: Context<'_>, partial: String) -> impl Stream<Item = Member> {
    futures::stream::iter(match ctx.guild() {
        Some(guild) => match guild.members(ctx.discord(), None, None).await {
            Ok(members) => members,
            Err(why) => {
                log::error!("Failed to get guild members: {}", why);
                Vec::<Member>::new() 
            },
        },
        None => Vec::<Member>::new(),
    }).filter(move |m| futures::future::ready(m.display_name().starts_with(&partial)))
}