use poise::serenity_prelude::{Context, Message};

use crate::{
    types::Data,
    unwrap_or_return,
    utils::{bee_utils, helper_functions},
};

pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return,
    };

    let (webhook, beelate) = {
        let beezone_channels = data.beezone_channels.read().await;
        let beeified_users = data.beeified_users.read().await;
        if beezone_channels.contains_key(&msg.channel_id) && !msg.author.bot {
            // Unwrapping is fine as we have already verified it is in the map
            let beezone_channel = *beezone_channels.get(&msg.channel_id).unwrap();

            // Drop the original locks
            drop(beezone_channels);
            drop(beeified_users);

            // If we are past the timestamp
            if beezone_channel.timestamp < msg.timestamp {
                let mut beezone_channels = data.beezone_channels.write().await;
                beezone_channels.remove(&msg.channel_id);
                return;
            }

            (
                unwrap_or_return!(
                    helper_functions::get_webhook(ctx, data, &msg.channel_id).await,
                    "Error getting webhook"
                ),
                beezone_channel.beelate,
            )
        } else if beeified_users.contains_key(&msg.author.id) {
            let beeified_user = *beeified_users.get(&msg.author.id).unwrap();

            // Drop the original locks
            drop(beezone_channels);
            drop(beeified_users);

            if beeified_user.timestamp < msg.timestamp {
                let mut beeified_users = data.beeified_users.write().await;
                beeified_users.remove(&msg.author.id);
                return;
            }

            (
                unwrap_or_return!(
                    helper_functions::get_webhook(ctx, data, &msg.channel_id).await,
                    "Error getting webhook"
                ),
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
