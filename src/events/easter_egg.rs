use crate::{types::data::Data, unwrap_or_return, Error};
use poise::{
    serenity_prelude::{Context, Message},
    FrameworkContext,
};
use rand::Rng;

pub async fn message(
    ctx: &Context,
    msg: &Message,
    data: &Data,
    framework_context: &FrameworkContext<'_, Data, Error>,
) {
    if msg
        .content
        .contains(&format!("<@{}>", framework_context.bot_id.0))
    {
        if rand::thread_rng().gen_bool(0.1) {
            let gif = unwrap_or_return!(
                sqlx::query!(
                    r#"SELECT content FROM ttc_easter_egg_gifs ORDER BY RANDOM() LIMIT 1"#
                )
                .fetch_one(&*data.pool)
                .await,
                "Error getting a GIF from the DB"
            )
            .content;
            unwrap_or_return!(
                msg.channel_id.send_message(ctx, |m| m.content(gif)).await,
                "Error sending GIF"
            );
        }
    }
}
