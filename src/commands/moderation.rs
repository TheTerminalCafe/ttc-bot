use crate::{
    types::{Context, Error},
    utils::{
        bee_utils::{BeeifiedUser, BeezoneChannel},
        helper_functions::{check_duration, format_duration, reply},
    },
};
use chrono::{Duration, Utc};
use poise::serenity_prelude::{Member, Timestamp, UserId};

/// Ban a member
///
/// Command to ban a member
/// ``ban [member] [dmd] [reason (optional)]``
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
    #[description = "Days of messages to delete"]
    #[min = 0]
    #[max = 7]
    dmd: u8,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    // Make sure people do not ban themselves
    if member.user == *ctx.author() {
        reply::input_error(
            &ctx,
            "That's a bad idea",
            "You should not try to ban yourself.",
        )
        .await?;
        return Ok(());
    }

    // Ban the person depending on if a reason was supplied
    match reason {
        Some(reason) => {
            member.ban_with_reason(ctx.discord(), dmd, reason).await?;
        }
        None => {
            member.ban(ctx.discord(), dmd).await?;
        }
    }

    reply::mod_punish(
        &ctx,
        "Banhammer has been swung.",
        &format!("{} has been banned.", member.user.tag()),
    )
    .await?;

    Ok(())
}

/// Ban a member (using the user id)
///
/// Command to ban a member
/// ``ban [user_id] [dmd] [reason (optional)]``
#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    required_permissions = "BAN_MEMBERS",
    guild_only
)]
pub async fn idban(
    ctx: Context<'_>,
    #[description = "Id of the user to silent ban"] user_id: UserId,
    #[description = "Days of messages to delete"] dmd: u8,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    if user_id == ctx.author().id {
        reply::input_error(
            &ctx,
            "That's a bad idea",
            "You should not try to ban yourself.",
        )
        .await?;
        return Ok(());
    }

    match reason {
        Some(reason) => {
            ctx.guild_id()
                .unwrap()
                .ban_with_reason(ctx.discord(), user_id, dmd, reason)
                .await?;
        }
        None => {
            ctx.guild_id()
                .unwrap()
                .ban(ctx.discord(), user_id, dmd)
                .await?;
        }
    }

    reply::mod_punish(
        &ctx,
        "Banhammer has been swung",
        &format!("{} has been banned.", user_id),
    )
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
        reply::input_error(
            &ctx,
            "I doubt there is a need for that",
            "Why are you trying to unban yourself, why?",
        )
        .await?;
        return Ok(());
    }

    ctx.guild_id().unwrap().unban(&ctx.discord(), user).await?;

    let tag = user.to_user(ctx.discord()).await?.tag();
    reply::mod_success(
        &ctx,
        "User forgiven",
        &format!("User {} has been unbanned", tag),
        false,
    )
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
        reply::input_error(
            &ctx,
            "That's a bad idea",
            "You really should not try to kick yourself.",
        )
        .await?;
        return Ok(());
    }

    match reason {
        Some(r) => member.kick_with_reason(ctx.discord(), &r).await?,
        None => member.kick(ctx.discord()).await?,
    }

    reply::mod_punish(
        &ctx,
        "The boot of justice has decided",
        &format!(
            "{} kicked. I hope justice has been made.",
            member.user.tag()
        ),
    )
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
        reply::input_error(
            &ctx,
            "That's a bad idea",
            "If you don't want to speak you can, you know, just not do that.",
        )
        .await?;
        return Ok(());
    }

    let duration = Duration::from_std(humantime::parse_duration(&duration_str)?)?;
    check_duration(duration, 28)?;
    member
        .disable_communication_until_datetime(
            ctx.discord(),
            Timestamp::parse(&(Utc::now() + duration).to_rfc3339())?,
        )
        .await?;

    reply::mod_punish(
        &ctx,
        "User timed out",
        &format!(
            "User {} timed out for {}",
            member.user.tag(),
            format_duration(&duration)
        ),
    )
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
        reply::input_error(
            &ctx,
            "It's useless to delete 0 messages",
            "Why would you want to do that?",
        )
        .await?;
        return Ok(());
    }

    if amount > 100 {
        reply::input_warn(
            &ctx,
            "n't delete over 100 messages",
            "Setting amount to 100",
        )
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

    reply::mod_success(
        &ctx,
        "Deleted",
        &format!("Deleted {} messages", amount),
        true,
    )
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
    let duration = Duration::from_std(humantime::parse_duration(&duration_str)?)?;
    // ~110 years; it's mainly here to prevent the bot from panicking
    check_duration(duration, 40000)?;
    let timestamp: Timestamp = (Utc::now() + duration).into();

    if user.user.bot {
        reply::input_error(&ctx, "That's a bad idea", "Bots can't be beeified.").await?;
        return Ok(());
    }

    let mut beeified_users = ctx.data().beeified_users.write().await;

    if beeified_users.contains_key(&user.user.id) {
        reply::input_error(&ctx, "Already beeified", "This user is already beeified.").await?;
        return Ok(());
    }

    beeified_users.insert(user.user.id, BeeifiedUser::new(timestamp, beelate));

    reply::mod_success(
        &ctx,
        "Beeified",
        &format!(
            "User <@{}> beeified for {}",
            user.user.id,
            format_duration(&duration)
        ),
        false,
    )
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
    let mut beeified_users = ctx.data().beeified_users.write().await;

    if !beeified_users.contains_key(&user.user.id) {
        reply::input_error(&ctx, "Not beeified", "This user is not beeified").await?;
        return Ok(());
    }

    beeified_users.remove(&user.user.id);

    reply::mod_success(
        &ctx,
        "Unbeeified",
        &format!("User <@{}> unbeeified", user.user.id),
        false,
    )
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
    let mut beezone_channels = ctx.data().beezone_channels.write().await;

    if beezone_channels.contains_key(&ctx.channel_id()) {
        reply::input_error(&ctx, "Already beezoned", "This channel is already beezoned").await?;
        return Ok(());
    }
    let duration = Duration::from_std(humantime::parse_duration(&duration_str)?)?;
    // ~110 years; it's mainly here to prevent the bot from panicking
    check_duration(duration, 40000)?;
    let timestamp: Timestamp = (Utc::now() + duration).into();

    beezone_channels.insert(ctx.channel_id(), BeezoneChannel::new(timestamp, beelate));

    reply::mod_success(
        &ctx,
        "Beezoned",
        &format!(
            "Channel <#{}> beezoned for {}",
            ctx.channel_id(),
            format_duration(&duration)
        ),
        false,
    )
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
    let mut beezone_channels = ctx.data().beezone_channels.write().await;

    if !beezone_channels.contains_key(&ctx.channel_id()) {
        reply::input_error(&ctx, "Not beezoned", "This channel is not beezoned.").await?;
        return Ok(());
    }

    beezone_channels.remove(&ctx.channel_id());

    reply::mod_success(
        &ctx,
        "Unbeezoned",
        &format!("Channel <#{}> unbeezoned", ctx.channel_id()),
        false,
    )
    .await?;

    Ok(())
}
