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
*/

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

/*
#[command]
#[num_args(1)]
#[description(
    "Command for creating a verification prompt using the role id provided within the config"
)]
async fn create_verification(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let channel_id = args.parse::<ChannelId>()?;

    channel_id
        .send_message(ctx, |m| {
            m.embed(|e| e.color(Color::FOOYOO).title("Be sure to follow the rules!"))
                .components(|c| {
                    c.create_action_row(|a| {
                        a.create_button(|b| {
                            b.label("Click here to finish verification")
                                .custom_id("ttc-bot-verification-button")
                                .style(ButtonStyle::Primary)
                        })
                    })
                })
        })
        .await?;

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Verification created."),
        Some(&format!("Verification prompt created in <#{}>", channel_id)),
        Some(Color::FOOYOO),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[description("Command for creating a selfrole menu")]
#[usage("<channel> <roles>")]
#[min_args(2)]
async fn create_selfroles(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Get the channel and guild ids
    let channel_id = args.single::<ChannelId>()?;
    let guild_id = msg.guild_id.unwrap();

    // Create the selection menu
    let mut menu = CreateSelectMenu::default();
    menu.custom_id("ttc-bot-self-role-menu");

    // Create the list for the roles
    let mut role_list: Vec<Role> = Vec::new();

    // Get the roles
    for arg in args.iter() {
        let arg: String = arg?;
        let role_id = arg.parse::<RoleId>()?;
        let roles = guild_id.roles(ctx).await?;
        if roles.contains_key(&role_id) {
            let role = roles[&role_id].clone();
            role_list.push(role);
        } else {
            embed_msg(
                ctx,
                &msg.channel_id,
                Some(&format!("Invalid role: <@{}>, {}", role_id, role_id)),
                None,
                Some(Color::RED),
                None,
            )
            .await?;
        }
    }

    // Make sure some valid roles were procided
    if role_list.len() == 0 {
        return command_error!("None of the provided roles were valid.");
    }

    // Set the menu values properly
    menu.min_values(0);
    menu.max_values(role_list.len() as u64);

    // Create the options for the roles
    menu.options(|m| {
        for role in role_list {
            m.create_option(|o| o.label(role.name).value(role.id));
        }
        m
    });

    // Create the menu in the specified channel
    channel_id
        .send_message(ctx, |m| {
            m.components(|c| c.create_action_row(|a| a.add_select_menu(menu)))
                .embed(|e| e.title("Manage your self roles here").color(Color::PURPLE))
        })
        .await?;

    // Reply to the user
    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Self role selection menu created"),
        Some(&format!(
            "Self role selection menu created in <#{}>.",
            channel_id
        )),
        Some(Color::FOOYOO),
        None,
    )
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
