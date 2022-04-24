use crate::{
    command_error, get_config,
    types::{Context, Error},
    utils::helper_functions::embed_msg,
};
use chrono::{Duration, Utc};
use poise::serenity_prelude::{Color, RoleId, User};

#[poise::command(
    slash_command,
    prefix_command,
    category = "Moderation",
    check = "is_mod",
    guild_only
)]
async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: User,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    // Get the user mentioned in the command

    // Make sure people do not ban themselves
    if user == *ctx.author() {
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
            ctx.guild()
                .unwrap()
                .ban_with_reason(ctx.discord(), &user, 0, &reason)
                .await?;
        }
        None => {
            ctx.guild().unwrap().ban(ctx.discord(), &user, 0).await?;
        }
    }

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Banhammer has been swung.")
                .description(format!("{} has been banned.", user.tag()))
                .color(Color::RED)
        })
        .ephemeral(true)
    })
    .await?;

    Ok(())
}

/*#[command]
#[num_args(1)]
#[required_permissions(BAN_MEMBERS)]
async fn pardon(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let user = args.single::<UserId>()?.to_user(ctx).await?;

    if user == msg.author {
        embed_msg(
            ctx,
            &msg.channel_id,
            Some("I doubt there is a need for that"),
            Some("Why are you trying to unban yourself, why?"),
            Some(Color::DARK_RED),
            None,
        )
        .await?;
        return Ok(());
    }

    let guild_id = msg.guild_id.unwrap();

    guild_id.unban(ctx, user.id).await?;

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("User forgiven"),
        Some(&format!("User {} has been unbanned", user.tag())),
        Some(Color::FOOYOO),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[min_args(1)]
#[max_args(2)]
#[required_permissions(KICK_MEMBERS)]
async fn kick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let user = args.single::<UserId>()?.to_user(ctx).await?;

    if user == msg.author {
        embed_msg(
            ctx,
            &msg.channel_id,
            Some("That's a bad idea."),
            Some("You really should not try to kick yourself."),
            Some(Color::DARK_RED),
            None,
        )
        .await?;
        return Ok(());
    }

    let guild_id = msg.guild_id.unwrap();

    match args.parse::<String>() {
        Ok(reason) => {
            guild_id
                .kick_with_reason(ctx, user.clone(), &reason)
                .await?
        }
        Err(_) => guild_id.kick(ctx, user.clone()).await?,
    };

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("The boot of justice has decided."),
        Some(&format!(
            "{} kicked. I hope justice has been made.",
            user.tag()
        )),
        Some(Color::RED),
        None,
    )
    .await?;

    Ok(())
}

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
}

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

async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {
    let config = get_config!(ctx.data(), {
        return Err(Error::from("Database error.".to_string()));
    });

    Ok(match ctx.author_member().await {
        Some(member) => member.roles.contains(&RoleId(config.moderator_role as u64)),
        None => false,
    })
}
