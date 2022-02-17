use crate::{
    command_error, get_config, typemap::types::PgPoolType, utils::helper_functions::embed_msg,
};
use serenity::{
    builder::CreateSelectMenu,
    client::Context,
    framework::standard::{
        macros::{check, command, group},
        Args, CommandResult, Reason,
    },
    model::{
        channel::Message,
        guild::Role,
        id::{ChannelId, RoleId, UserId},
        interactions::message_component::ButtonStyle,
    },
    utils::Color,
};

#[group]
#[prefix("mod")]
#[allowed_roles("Moderator")]
#[checks(is_mod)]
#[only_in(guilds)]
#[commands(ban, kick, create_verification, create_selfroles)]
struct Moderation;

#[command]
#[min_args(1)]
#[max_args(2)]
#[required_permissions(BAN_MEMBERS)]
async fn ban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Get the user mentioned in the command
    let user = match match args.single::<UserId>() {
        Ok(user_id) => user_id,
        Err(why) => return command_error!(format!("Invalid user id: {}", why)),
    }
    .to_user(ctx)
    .await
    {
        Ok(user) => user,
        Err(why) => return command_error!(format!("Invalid user: {}", why)),
    };

    // Make sure people do not ban themselves
    if user == msg.author {
        embed_msg(
            ctx,
            &msg.channel_id,
            Some("That's a bad idea."),
            Some("You really should not try to ban yourself."),
            Some(Color::DARK_RED),
            None,
        )
        .await?;
        return Ok(());
    }

    // Also make sure a guild id is available
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return command_error!("No guild id available!"),
    };

    // Ban the person depending on if a reason was supplied
    match args.parse::<String>() {
        Ok(reason) => match guild_id.ban_with_reason(ctx, user.clone(), 2, reason).await {
            Ok(_) => (),
            Err(why) => return command_error!(format!("Error banning user: {}", why)),
        },
        Err(_) => match guild_id.ban(ctx, user.clone(), 2).await {
            Ok(_) => (),
            Err(why) => return command_error!(format!("Error banning user: {}", why)),
        },
    };

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Banhammer has been swung"),
        Some(&format!(
            "{} banned. I hope justice has been made.",
            user.tag()
        )),
        Some(Color::RED),
        None,
    )
    .await?;

    Ok(())
}

#[command]
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

    match guild_id.unban(ctx, user.id).await {
        Ok(_) => (),
        Err(why) => {
            return command_error!(format!("Unable to unban user {}: {}", user.tag(), why));
        }
    }

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("User unbanned"),
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
        Ok(reason) => match guild_id.kick_with_reason(ctx, user.clone(), &reason).await {
            Ok(_) => (),
            Err(why) => return command_error!(format!("Error kicking user: {}", why)),
        },
        Err(_) => match guild_id.kick(ctx, user.clone()).await {
            Ok(_) => (),
            Err(why) => return command_error!(format!("Error kicking user: {}", why)),
        },
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
#[min_args(2)]
async fn create_selfroles(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel_id = args.single::<ChannelId>()?;
    let guild_id = msg.guild_id.unwrap();

    let mut menu = CreateSelectMenu::default();
    menu.custom_id("ttc-bot-self-role-menu");

    let mut role_list: Vec<Role> = Vec::new();

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

    if role_list.len() == 0 {
        return command_error!("None of the provided roles were valid.");
    }

    menu.min_values(0);
    menu.max_values(role_list.len() as u64);

    menu.options(|m| {
        for role in role_list {
            m.create_option(|o| o.label(role.name).value(role.id));
        }
        m
    });

    channel_id
        .send_message(ctx, |m| {
            m.components(|c| c.create_action_row(|a| a.add_select_menu(menu)))
                .embed(|e| e.title("Manage your self roles here").color(Color::PURPLE))
        })
        .await?;

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
}

#[check]
async fn is_mod(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let config = get_config!(ctx, {
        return Err(Reason::Log("Database error.".to_string()));
    });

    if match msg
        .author
        .has_role(
            ctx,
            match msg.guild_id {
                Some(id) => id,
                None => {
                    return Err(Reason::UserAndLog {
                        user: "Not in a server!".to_string(),
                        log: "Moderation command called outside a server".to_string(),
                    })
                }
            },
            RoleId(config.moderator_role as u64),
        )
        .await
    {
        Ok(result) => result,
        Err(why) => {
            log::error!("Failed to get user roles: {}", why);
            false
        }
    } {
        return Ok(());
    }

    Err(Reason::Log("No mod role found".to_string()))
}
