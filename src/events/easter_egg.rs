use crate::types::{Data, Error};
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
            let gif = match sqlx::query!(
                r#"SELECT content FROM ttc_easter_egg_gifs ORDER BY RANDOM() LIMIT 1"#
            )
            .fetch_one(&data.pool)
            .await
            {
                Ok(gif) => gif.content,
                Err(why) => {
                    log::error!("Error getting a GIF from the DB: {}", why);
                    return;
                }
            };
            match msg.channel_id.send_message(ctx, |m| m.content(gif)).await {
                Ok(_) => (),
                Err(why) => {
                    log::error!("Error sending GIF: {}", why);
                    return;
                }
            }
        }
    }
}
