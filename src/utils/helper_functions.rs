use poise::serenity_prelude::{ChannelId, Color, Context, CreateEmbed, Message, Webhook};

use crate::{types::data::Data, Error};
use std::time::Duration;

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

// May be useful later, but is not right now
#[allow(dead_code)]
pub async fn alert_mods(ctx: &Context, embed: CreateEmbed, data: &Data) -> Result<(), Error> {
    let mod_role = data.config.moderator_role().await?;
    for channel in &data.config.conveyance_channel().await? {
        ChannelId(*channel as u64)
            .send_message(ctx, |m| {
                m.content(format!("<@&{}>", mod_role))
                    .set_embed(embed.clone())
            })
            .await?;
    }

    Ok(())
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
                .execute(&*data.pool)
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
                .execute(&*data.pool)
                .await?;
            }
            log::info!("Created missing webhook for channel {}", channel_id);
            webhook
        }
    })
}
