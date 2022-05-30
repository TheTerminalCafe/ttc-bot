use poise::serenity_prelude::{Context, Message};

use crate::{types::Data, utils::bee_utils};

pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return,
    };

    let (webhook, beelate) = {
        let mut beezone_channels = data.beezone_channels.lock().await;
        let mut beeified_users = data.beeified_users.lock().await;
        if beezone_channels.contains_key(&msg.channel_id) && !msg.author.bot {
            // Unwrapping is fine as we have already verified it is in the map
            let beezone_channel = beezone_channels.get(&msg.channel_id).unwrap();

            // If we are past the timestamp
            if beezone_channel.timestamp < msg.timestamp {
                beezone_channels.remove(&msg.channel_id);
                return;
            }

            (
                match data.webhooks.lock().await.get(&msg.channel_id) {
                    Some(webhook) => webhook.clone(),
                    None => {
                        log::error!("No webhook found for channel {}, if this channel is a new one please restart the bot.", msg.channel_id);
                        return;
                    }
                },
                beezone_channel.beelate,
            )
        } else if beeified_users.contains_key(&msg.author.id) {
            let beeified_user = beeified_users.get(&msg.author.id).unwrap();

            if beeified_user.timestamp < msg.timestamp {
                beeified_users.remove(&msg.author.id);
                return;
            }

            (
                match data.webhooks.lock().await.get(&msg.channel_id) {
                    Some(webhook) => webhook.clone(),
                    None => {
                        log::error!("No webhook found for channel {}, if this channel is a new one please restart the bot.", msg.channel_id);
                        return;
                    }
                },
                beeified_user.beelate,
            )
        } else {
            return;
        }
    };

    {
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
    }

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
