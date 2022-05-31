use crate::{
    types::{Context, Error},
    utils::{
        bee_utils::{BeeifiedUser, BeezoneChannel},
        helper_functions::format_duration,
    },
};
use chrono::{Duration, Utc};
use poise::serenity_prelude::{Color, Member, Timestamp, UserId};

/// Ban an member
///
/// Command to ban a member
/// ``ban [member] [reason (optional)]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "BAN_MEMBERS",
    guild_only
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] member: Member,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    // Make sure people do not ban themselves
    if member.user == *ctx.author() {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("That's a bad idea")
                    .description("You should not try to ban yourself.")
                    .color(Color::RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    // Ban the person depending on if a reason was supplied
    match reason {
        Some(reason) => {
            member.ban_with_reason(ctx.discord(), 0, reason).await?;
        }
        None => {
            member.ban(ctx.discord(), 0).await?;
        }
    }

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Banhammer has been swung.")
                .description(format!("{} has been banned.", member.user.tag()))
                .color(Color::RED)
        })
    })
    .await?;

    Ok(())
}

/// Unban an user
///
/// Command to unban an user by id
/// ``unban [user]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "BAN_MEMBERS",
    guild_only
)]
pub async fn pardon(
    ctx: Context<'_>,
    #[description = "The user id to pardon"] user: UserId,
) -> Result<(), Error> {
    let author = ctx.author();

    if author.id == user {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("I doubt there is a need for that")
                    .description("Why are you trying to unban yourself, why?")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    ctx.guild_id().unwrap().unban(&ctx.discord(), user).await?;

    let tag = user.to_user(ctx.discord()).await?.tag();
    ctx.send(|m| {
        m.embed(|e| {
            e.title("User forgiven")
                .description(format!("User {} has been unbanned", tag))
                .color(Color::FOOYOO)
        })
    })
    .await?;
    Ok(())
}

/// Kick a member
///
/// Command to kick a member
/// ``kick [member] [reason (optional)]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "KICK_MEMBERS",
    guild_only
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "User to kick"] member: Member,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    let author: Member = ctx.author_member().await.unwrap();
    if author.user == member.user {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("That's a bad idea.")
                    .description("You really should not try to kick yourself.")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    match reason {
        Some(r) => member.kick_with_reason(ctx.discord(), &r).await?,
        None => member.kick(ctx.discord()).await?,
    }

    ctx.send(|m| {
        m.embed(|e| {
            e.title("The boot of justice has decided.")
                .description(format!(
                    "{} kicked. I hope justice has been made.",
                    member.user.tag()
                ))
                .color(Color::RED)
        })
    })
    .await?;
    Ok(())
}

/// Timeout a member
///
/// Command to timeout a member
/// ``timeout [member] [duration]``
///
/// ``duration`` is a human-readable string like \
/// ``1h``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MODERATE_MEMBERS",
    guild_only
)]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "The member to timeout"] mut member: Member,
    #[description = "Time to timeout user"]
    #[rename = "duration"]
    duration_str: String,
) -> Result<(), Error> {
    let author: Member = ctx.author_member().await.unwrap();
    if author.user == member.user {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("That's a bad idea.")
                    .description("If you don't want to speak you can, you know, just not do that.")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    let duration = Duration::from_std(parse_duration::parse(&duration_str)?)?;
    member
        .disable_communication_until_datetime(
            ctx.discord(),
            Timestamp::parse(&(Utc::now() + duration).to_rfc3339())?,
        )
        .await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("User timed out")
                .description(format!(
                    "User {} timed out for {}",
                    member.user.tag(),
                    format_duration(&duration)
                ))
                .color(Color::RED)
        })
    })
    .await?;

    Ok(())
}

/// Purge messages
///
/// Delete a certain amount of messages (max 100)
/// ``purge [amount]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MANAGE_MESSAGES",
    guild_only
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Amount"] mut amount: u64,
) -> Result<(), Error> {
    if amount == 0 {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("It's useless to delete 0 messages")
                    .description("Why would you want to do that?")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    if amount > 100 {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Can't delete over 100 messages")
                    .description("Setting amount to 100")
                    .color(Color::RED)
            })
            .ephemeral(true)
        })
        .await?;
        amount = 100;
    }
    let messages = ctx
        .channel_id()
        .messages(ctx.discord(), |b| b.before(ctx.id()).limit(amount))
        .await?;

    ctx.channel_id()
        .delete_messages(ctx.discord(), messages)
        .await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Deleted")
                .description(format!("Deleted {} messages", amount))
                .color(Color::FOOYOO)
        })
        .ephemeral(true)
    })
    .await?;
    Ok(())
}

/// Beeify a member
///
/// Command to beeify a member
/// ``beeify [member] [duration] [beelate]``
///
/// ``duration`` is a human-readable string like \
/// ``1h``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MODERATE_MEMBERS",
    guild_only
)]
pub async fn beeify(
    ctx: Context<'_>,
    #[description = "User to beeify"] user: Member,
    #[description = "The time to beeify the user for"]
    #[rename = "duration"]
    duration_str: String,
    #[description = "Whether to use beelate or not"] beelate: bool,
) -> Result<(), Error> {
    let duration = Duration::from_std(parse_duration::parse(&duration_str)?)?;
    let timestamp: Timestamp = (Utc::now() + duration).into();

    if user.user.bot {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("That's a bad idea.")
                    .description("Bots can't be beeified.")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    let mut beeified_users = ctx.data().beeified_users.lock().await;

    if beeified_users.contains_key(&user.user.id) {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Already beeified")
                    .description("This user is already beeified")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    beeified_users.insert(user.user.id, BeeifiedUser::new(timestamp, beelate));

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Beeified")
                .description(format!(
                    "User {} beeified for {}",
                    user.user.tag(),
                    format_duration(&duration)
                ))
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

/// Unbeeify a member
///
/// Command to unbeeify a member
/// ``unbeeify [member]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MODERATE_MEMBERS",
    guild_only
)]
pub async fn unbeeify(
    ctx: Context<'_>,
    #[description = "User to unbeeify"] user: Member,
) -> Result<(), Error> {
    let mut beeified_users = ctx.data().beeified_users.lock().await;

    if !beeified_users.contains_key(&user.user.id) {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Not beeified")
                    .description("This user is not beeified")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    beeified_users.remove(&user.user.id);

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Unbeeified")
                .description(format!("User {} unbeeified", user.user.tag()))
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

/// Beezone.
///
/// Turn the current channel into instant chaos.
/// ``beezone``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MODERATE_MEMBERS",
    guild_only
)]
pub async fn beezone(
    ctx: Context<'_>,
    #[description = "The time to cause chaos for"]
    #[rename = "duration"]
    duration_str: String,
    #[description = "Whether to use beelate or not"] beelate: bool,
) -> Result<(), Error> {
    let mut beezone_channels = ctx.data().beezone_channels.lock().await;

    if beezone_channels.contains_key(&ctx.channel_id()) {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Already beezoned")
                    .description("This channel is already beezoned")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }
    let duration = Duration::from_std(parse_duration::parse(&duration_str)?)?;
    let timestamp: Timestamp = (Utc::now() + duration).into();

    beezone_channels.insert(ctx.channel_id(), BeezoneChannel::new(timestamp, beelate));

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Beezoned")
                .description(format!(
                    "Channel {} beezoned for {}",
                    ctx.channel_id(),
                    format_duration(&duration)
                ))
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

/// Unbeezone
///
/// Turn the current channel back into normal.
/// ``unbeezone``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "MODERATE_MEMBERS",
    guild_only
)]
pub async fn unbeezone(ctx: Context<'_>) -> Result<(), Error> {
    let mut beezone_channels = ctx.data().beezone_channels.lock().await;

    if !beezone_channels.contains_key(&ctx.channel_id()) {
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Not beezoned")
                    .description("This channel is not beezoned")
                    .color(Color::DARK_RED)
            })
            .ephemeral(true)
        })
        .await?;
        return Ok(());
    }

    beezone_channels.remove(&ctx.channel_id());

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Unbeezoned")
                .description(format!("Channel {} unbeezoned", ctx.channel_id()))
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}
