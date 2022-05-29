use poise::serenity_prelude::{Context, Message};

use crate::{types::Data, utils::bee_utils};

pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return,
    };

    let (webhook, beelate) = {
        let mut beezone_channels = data.beezone_channels.lock().await;
        if beezone_channels.contains_key(&msg.channel_id) && msg.author.discriminator != 0 {
            let beezone_channel = beezone_channels.get_mut(&msg.channel_id).unwrap();
            if beezone_channel.timestamp > msg.timestamp {
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
                match &beezone_channel.webhook {
                    Some(webhook) => match webhook.delete(ctx).await {
                        Ok(_) => (),
                        Err(why) => {
                            log::error!("Error deleting webhook: {}", why);
                        }
                    },
                    None => log::warn!("No webhook found for beezone channel"),
                }
                beezone_channels.remove(&msg.channel_id);
                return;
            }
            (
                match &beezone_channel.webhook {
                    Some(webhook) => webhook.clone(),
                    None => {
                        let webhook = match msg.channel_id.create_webhook(ctx, "Beezone").await {
                            Ok(webhook) => webhook,
                            Err(why) => {
                                log::error!("Failed to create webhook: {}", why);
                                return;
                            }
                        };

                        beezone_channel.webhook = Some(webhook.clone());
                        webhook
                    }
                },
                beezone_channel.beelate,
            )
        } else {
            let mut beeified_users = data.beeified_users.lock().await;
            if beeified_users.contains_key(&msg.author.id) {
                let beeified_user = beeified_users.get_mut(&msg.author.id).unwrap();
                if beeified_user.timestamp > msg.timestamp {
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
                    for (_, webhook) in &beeified_user.webhooks {
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

                if beeified_user.webhooks.contains_key(&msg.channel_id) {
                    (
                        beeified_user.webhooks[&msg.channel_id].clone(),
                        beeified_user.beelate,
                    )
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

                    beeified_user.webhooks.insert(msg.channel_id, webhook);
                    (
                        beeified_user.webhooks[&msg.channel_id].clone(),
                        beeified_user.beelate,
                    )
                }
            } else {
                return;
            }
        }
    };

    let content = if beelate {
        bee_utils::beelate(&msg.content)
    } else {
        bee_utils::get_bee_line(None)
    };
    let name = msg
        .author
        .nick_in(ctx, guild_id)
        .await
        .unwrap_or(msg.author.name.clone());

    match webhook
        .execute(ctx, true, |w| {
            w.content(content)
                .avatar_url(msg.author.face())
                .username(name)
        })
        .await
    {
        Ok(_) => (),
        Err(why) => {
            log::error!("Failed to execute webhook: {}", why);
        }
    }
}
