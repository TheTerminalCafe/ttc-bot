use futures::StreamExt;
use poise::serenity_prelude::{Color, User};

use crate::types::{Context, Error};
use std::iter::Iterator;
// ----------------------
// General group commands
// ----------------------

#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong").await?;

    Ok(())
}

#[poise::command(slash_command, context_menu_command = "User info")]
pub async fn userinfo(ctx: Context<'_>, #[description = "User"] user: User) -> Result<(), Error> {
    ctx.defer().await?;

    let (nickname, joined_at, roles) = match ctx.guild() {
        Some(guild) => {
            match guild.member(ctx.discord(), user.id).await {
                Ok(member) => {
                    let nick = member.nick.clone().unwrap_or("None".to_string());
                    let joined_at = match member.joined_at {
                        Some(joined_at) => format!("{}", joined_at),
                        None => "N/A".to_string(),
                    };
                    let mut roles = match member.roles(ctx.discord()) {
                        Some(roles) => roles
                            .iter()
                            .map(|role| format!("<@&{}>, ", role.id))
                            .collect::<String>(),
                        None => "None".to_string(),
                    };
                    // Remove trailing comma and space
                    roles.pop();
                    roles.pop();

                    // Make sure it isn't empty
                    if roles == "" {
                        roles = "None".to_string()
                    }
                    (nick, joined_at, roles)
                }
                Err(_) => ("N/A".to_string(), "N/A".to_string(), "N/A".to_string()),
            }
        }
        None => ("N/A".to_string(), "N/A".to_string(), "N/A".to_string()),
    };

    ctx.send(|m| {
        m.embed(|e| {
            e.author(|a| a.name(user.tag()).icon_url(user.face()))
                .field("User ID", user.id.0, true)
                .field("Nickname", nickname, true)
                .field("Created At", user.id.created_at(), false)
                .field("Joined At", joined_at, false)
                .field("Roles", roles, false)
                .field("Icon URL", user.face(), false)
                .color(Color::BLITZ_BLUE)
        })
        .ephemeral(true)
    })
    .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    context_menu_command = "Harold percentage",
    prefix_command,
    guild_only
)]
pub async fn harold(
    ctx: Context<'_>,
    #[description = "User to calculate harold percentage of"] user: User,
) -> Result<(), Error> {
    ctx.defer().await?;
    let mut user_messages: u64 = 0;
    let mut user_harold_messages: u64 = 0;
    let mut global_messages: u64 = 0;
    let mut global_harold_messages: u64 = 0;
    for (channel_id, _) in ctx.guild().unwrap().channels {
        let mut messages = channel_id.messages_iter(ctx.discord()).boxed();
        while let Some(message) = messages.next().await {
            match message {
                Ok(message) => {
                    global_messages += 1;
                    if message.content.contains(":helpmeplz:") {
                        global_harold_messages += 1;
                    }
                    if message.author == user {
                        user_messages += 1;
                        if message.content.contains(":helpmeplz:") {
                            user_harold_messages += 1;
                        }
                    }
                }
                Err(why) => log::error!("Something went wrong when getting message: {}", why),
            }
        }
    }

    ctx.say(format!(
        "Messages: {}, Harold messages: {}, Percentage: {}, Global messages: {}, Global harold messages: {}, Global percentage: {}",
        user_messages,
        user_harold_messages,
        (user_harold_messages as f64 / user_messages as f64) * 100.0,
        global_messages,
        global_harold_messages,
        (global_harold_messages as f64 / global_messages as f64) * 100.0,
    ))
    .await?;

    Ok(())
}
