use crate::{
    types::{Context, Error},
    utils::helper_functions::format_duration,
};
use chrono::{Duration, Utc};
use poise::serenity_prelude::{Color, Member, Timestamp, UserId};

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