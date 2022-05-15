use crate::types::{Context, Error};
use poise::serenity_prelude::{Color, Member, UserId};

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
    // Get the user mentioned in the command

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

/*
#[command]
#[min_args(2)]
async fn timeout(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let user_id = args.single::<UserId>()?;

    let duration_str = args.rest();
    let duration = match Duration::from_std(match parse_duration::parse(duration_str) {
        Ok(duration) => duration,
        Err(why) => {
            return command_error!("Error parsing duration: {}", why);
        }
    }) {
        Ok(duration) => duration,
        Err(why) => {
            return command_error!("Error parsing duration: {}", why);
        }
    };

    let guild_id = msg.guild_id.unwrap();

    let mut member = guild_id.member(ctx, user_id).await?;

    member
        .disable_communication_until_datetime(ctx, Utc::now() + duration)
        .await?;

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("User timed out"),
        Some(&format!(
            "User {} timed out for {}",
            member.user.tag(),
            duration
        )),
        Some(Color::RED),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[num_args(1)]
#[description("Delete multiple messages")]
#[required_permissions(MANAGE_MESSAGES)]
#[usage("<number of messages to delete>")]
async fn purge(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let amount = args.single::<u64>()?;

    if amount > 100 {
        return command_error!("Unable to delete more than 100 items");
    } else if amount == 0 {
        return command_error!("Unable to delete 0 items");
    }

    let messages = msg
        .channel_id
        .messages(ctx, |retriever| retriever.before(msg.id).limit(amount))
        .await?;

    msg.channel_id.delete_messages(ctx, messages).await?;

    let reply = msg
        .reply(
            ctx,
            format!(
                "Deleted {} messages. This message will self destruct in 5 seconds.",
                amount
            ),
        )
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    msg.channel_id
        .delete_messages(ctx, vec![msg.id, reply.id])
        .await?;

    Ok(())
}*/

/*async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {
    let config = get_config!(ctx.data(), {
        return Err(Error::from("Database error.".to_string()));
    });

    Ok(match ctx.author_member().await {
        Some(member) => member.roles.contains(&RoleId(config.moderator_role as u64)),
        None => false,
    })
}*/
