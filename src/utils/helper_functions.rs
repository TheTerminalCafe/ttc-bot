use chrono::{DateTime, Utc};
use poise::serenity_prelude::{
    ChannelId, Color, Context, CreateEmbed, CreateMessage, Message, User, Webhook,
};

use crate::types::{Data, Error};
use std::{sync::Arc, time::Duration};

// ----------------
// Helper functions
// ----------------

// Helper function for fast and easy embed messages
pub async fn embed_msg(
    ctx: &Context,
    channel_id: &ChannelId,
    title: Option<&str>,
    description: Option<&str>,
    color: Option<Color>,
    autodelete: Option<Duration>,
) -> Result<Message, Error> {
    let mut embed = CreateEmbed::default();

    match title {
        Some(title) => {
            embed.title(title);
        }
        None => (),
    }
    match description {
        Some(description) => {
            embed.description(description);
        }
        None => (),
    }
    match color {
        Some(color) => {
            embed.color(color);
        }
        None => (),
    }

    let msg = channel_id.send_message(ctx, |m| m.set_embed(embed)).await?;

    match autodelete {
        Some(duration) => {
            tokio::time::sleep(duration).await;
            msg.delete(ctx).await?;
        }
        None => (),
    }

    Ok(msg)
}

// Function for waiting for the author of msg to send a message
pub async fn wait_for_message(
    ctx: &Context,
    channel_id: &ChannelId,
    author: &User,
    timeout: Duration,
) -> Result<Arc<Message>, Error> {
    let message = match author.await_reply(ctx).timeout(timeout).await {
        Some(msg) => msg,
        None => {
            embed_msg(
                ctx,
                &channel_id,
                Some("Timeout!"),
                Some(&format!(
                    "No reply sent in {} minutes and {} seconds",
                    timeout.as_secs() / 60,
                    timeout.as_secs() % 60,
                )),
                Some(Color::RED),
                None,
            )
            .await?;
            return Err(Error::from("No reply received for problem description"));
        }
    };

    Ok(message)
}

// Helper function for asking for user input after a message from the bot
pub async fn get_message_reply<'a, F>(
    ctx: &Context,
    channel_id: &ChannelId,
    user: &User,
    question_msg_f: F,
    timeout: Duration,
    data: &Data,
) -> Result<String, Error>
where
    for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a>,
{
    // Ask the user first
    let question_msg = channel_id.send_message(ctx, question_msg_f).await?;

    // Get the reply message
    // The loops are for making sure there is at least some text content in the message
    let msg = loop {
        let new_msg = match wait_for_message(ctx, channel_id, user, timeout).await {
            Ok(msg) => msg,
            Err(why) => {
                let mut guard = data.users_currently_questioned.write().await;
                guard.retain(|uid| uid != &user.id);
                return Err(Error::from(format!(
                    "User took too long to respond: {}",
                    why
                )));
            }
        };
        if new_msg.content != "" {
            break new_msg;
        }
        embed_msg(
            ctx,
            channel_id,
            None,
            Some("Please send a message with text content."),
            Some(Color::RED),
            Some(Duration::from_secs(3)),
        )
        .await?;
    };

    // Get the message content
    let content = msg.content_safe(ctx);

    // Clean up messages
    match msg
        .channel_id
        .delete_messages(ctx, vec![question_msg.id, msg.id])
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Error deleting messages: {}", why),
    }

    // Return content
    Ok(content)
}

// May be useful later, but is not right now
#[allow(dead_code)]
pub async fn alert_mods(ctx: &Context, embed: CreateEmbed, data: &Data) -> Result<(), Error> {
    let mod_role = data.moderator_role().await?;
    for channel in &data.conveyance_channel().await? {
        ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.content(format!("<@&{}>", mod_role))
                    .set_embed(embed.clone())
            })
            .await?;
    }

    Ok(())
}

pub fn format_duration(dur: &chrono::Duration) -> String {
    let mut result = String::new();
    let mut raw = dur.num_seconds();
    let seconds = raw % 60;
    raw /= 60;
    let minutes = raw % 60;
    raw /= 60;
    let hours = raw % 24;
    raw /= 24;
    let days = raw;
    match days {
        0 => {}
        1 => result = format!("{} {} Day ", result, days),
        _ => result = format!("{} {} Days ", result, days),
    }
    match hours {
        0 => {}
        1 => result = format!("{} {} Hour ", result, hours),
        _ => result = format!("{} {} Hours ", result, hours),
    }
    match minutes {
        0 => {}
        1 => result = format!("{} {} Minute ", result, minutes),
        _ => result = format!("{} {} Minutes ", result, minutes),
    }
    match seconds {
        0 => {}
        1 => result = format!("{} {} Second", result, seconds),
        _ => result = format!("{} {} Seconds", result, seconds),
    }
    if result.len() == 0 {
        result = format!("0 Seconds");
    }
    result.trim_end().to_owned()
}

pub async fn get_webhook(
    ctx: &Context,
    data: &Data,
    channel_id: &ChannelId,
) -> Result<Webhook, Error> {
    let webhooks = data.webhooks.read().await;
    Ok(match webhooks.get(channel_id) {
        Some(webhook) => webhook.clone(),
        None => {
            drop(webhooks);
            let mut webhooks = data.webhooks.write().await;
            let webhook = channel_id
                .create_webhook(ctx, format!("ttc-bot fancy webhook {}", channel_id))
                .await?;
            webhooks.insert(channel_id.clone(), webhook.clone());
            // Update the webhook URLs in the DB
            sqlx::query!(r#"DELETE FROM ttc_webhooks"#)
                .execute(&data.pool)
                .await?;
            for (channel_id, webhook) in webhooks.iter() {
                sqlx::query!(
                    r#"INSERT INTO ttc_webhooks (channel_id, webhook_url) VALUES ($1, $2)"#,
                    channel_id.0 as i64,
                    match webhook.url() {
                        Ok(url) => url,
                        Err(why) => {
                            log::error!("Malformed webhook: {}", why);
                            continue;
                        }
                    }
                )
                .execute(&data.pool)
                .await?;
            }
            log::info!("Created missing webhook for channel {}", channel_id);
            webhook
        }
    })
}

pub fn format_datetime(timestamp: &DateTime<Utc>) -> String {
    format!(
        "{} ({})",
        timestamp.format("%d.%m.%Y at %H:%M:%S"),
        timestamp.timezone()
    )
    .to_owned()
}

pub fn check_duration(duration: chrono::Duration, max_days: i64) -> Result<(), Error> {
    if duration.num_days() > max_days {
        return Err(Error::from(format!(
            "Your specified number of days is over the maximum of {} days",
            max_days
        )));
    }
    Ok(())
}

pub mod reply {
    use crate::{ttc_reply_generate, Data, Error};
    use poise::serenity_prelude::Color;
    use poise::Context;

    // Only call it over ``ttc_reply_error`` etc. to enforce the usage of DB colors
    async fn ttc_reply<T>(
        ctx: &'_ Context<'_, Data, Error>,
        color: Color,
        ephemeral: bool,
        title: T,
        description: T,
        fields: Vec<(T, T, bool)>,
    ) -> Result<(), Error>
    where
        T: ToString,
    {
        ctx.send(|b| {
            b.embed(|e| {
                e.title(title)
                    .description(description)
                    .fields(fields)
                    .color(color)
            })
            .ephemeral(ephemeral)
        })
        .await?;
        Ok(())
    }

    // Used for warnings about user input (currently only used when over 100
    // messages are attempted to be deleted)
    ttc_reply_generate!(input_warn, Color::ORANGE, true);

    // When a user sends wrong/weird input (e.g. kicking themself)
    ttc_reply_generate!(input_error, Color::RED, true);

    // Default moderation color for punishments (e.g. banning/muting)
    ttc_reply_generate!(mod_punish, Color::FOOYOO, false);

    // Moderation color for success (e.g. message purges)
    ttc_reply_generate!(mod_success, Color::FOOYOO);

    // Admin color for success (e.g. create verification/selfroles)
    ttc_reply_generate!(admin_success, Color::FOOYOO, false);

    // Other Errors (e.g. EmojiCache already being updated)
    ttc_reply_generate!(general_error, Color::RED, true);

    // Normal Info about the Emoji cache
    ttc_reply_generate!(emoji_info, Color::FOOYOO, true);

    // When a Bee is trying to translate something
    ttc_reply_generate!(bee_translate_block, Color::KERBAL, true);

    // Translate color
    pub async fn translate<T>(
        ctx: &'_ Context<'_, Data, Error>,
        title: T,
        description: T,
        fields: Vec<(T, T)>,
    ) -> Result<(), Error>
    where
        T: ToString,
    {
        let color = ctx.data().translate().await;
        let fields = fields
            .into_iter()
            .map(|val| (val.0, val.1, false))
            .collect::<Vec<(T, T, bool)>>();
        ttc_reply(ctx, color, false, title, description, fields).await?;
        Ok(())
    }

    // When a Bee is trying to translate something
    ttc_reply_generate!(support_info, Color::FOOYOO, false);
}
