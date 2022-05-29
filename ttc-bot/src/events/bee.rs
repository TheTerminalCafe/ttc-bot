use poise::serenity_prelude::{Context, Message};

use crate::{types::Data, utils::bee_utils};

pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return,
    };

    let webhook = {
        let mut beeified_users = data.beeified_users.lock().await;
        if beeified_users.contains_key(&msg.author.id) {
            let (timestamp, webhooks) = beeified_users.get_mut(&msg.author.id).unwrap();
            if *timestamp > msg.timestamp {
                // Async message deletion
                let msg = msg.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    match msg.delete(ctx).await {
                        Ok(_) => (),
                        Err(why) => {
                            log::error!("Error deleting message: {}", why);
                        }
                    }
                });
            } else {
                for (_, webhook) in webhooks {
                    match webhook.delete(ctx).await {
                        Ok(_) => (),
                        Err(why) => {
                            log::error!("Error deleting webhook: {:?}", why);
                            return;
                        }
                    }
                }
                beeified_users.remove(&msg.author.id);
                return;
            }

            if webhooks.contains_key(&msg.channel_id) {
                webhooks[&msg.channel_id].clone()
            } else {
                let webhook = match msg
                    .channel_id
                    .create_webhook(
                        ctx,
                        &msg.author
                            .nick_in(ctx, guild_id)
                            .await
                            .unwrap_or(msg.author.name.clone()),
                    )
                    .await
                {
                    Ok(webhook) => webhook,
                    Err(why) => {
                        log::error!("Failed to create webhook: {}", why);
                        return;
                    }
                };

                webhooks.insert(msg.channel_id, webhook);
                webhooks[&msg.channel_id].clone()
            }
        } else {
            return;
        }
    };

    match webhook
        .execute(ctx, true, |w| {
            w.content(bee_utils::get_bee_line(None))
                .avatar_url(msg.author.face())
        })
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Failed to execute webhook: {}", why);
        }
    }
}
