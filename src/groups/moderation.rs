use crate::{
    typemap::{config::Config, types::PgPoolType},
    utils::helper_functions::embed_msg,
};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{check, command, group},
        Args, CommandError, CommandResult, Reason,
    },
    model::{
        channel::Message,
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
#[commands(ban, kick, create_verification)]
struct Moderation;

#[command]
#[min_args(1)]
#[max_args(2)]
#[required_permissions(BAN_MEMBERS)]
async fn ban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let user = match match args.parse::<UserId>() {
        Ok(user_id) => user_id,
        Err(why) => return Err(CommandError::from(format!("Invalid user id: {}", why))),
    }
    .to_user(ctx)
    .await
    {
        Ok(user) => user,
        Err(why) => return Err(CommandError::from(format!("Invalid user: {}", why))),
    };

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

    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return Err(CommandError::from("No guild id available!")),
    };

    match args.parse::<String>() {
        Ok(reason) => match guild_id.ban_with_reason(ctx, user.clone(), 2, reason).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error banning user: {}", why))),
        },
        Err(_) => match guild_id.ban(ctx, user.clone(), 2).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error banning user: {}", why))),
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
#[min_args(1)]
#[max_args(2)]
#[required_permissions(KICK_MEMBERS)]
async fn kick(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let user = match match args.parse::<UserId>() {
        Ok(user_id) => user_id,
        Err(why) => return Err(CommandError::from(format!("Invalid user id: {}", why))),
    }
    .to_user(ctx)
    .await
    {
        Ok(user) => user,
        Err(why) => return Err(CommandError::from(format!("Invalid user: {}", why))),
    };

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

    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return Err(CommandError::from("No guild id available!")),
    };

    match args.parse::<String>() {
        Ok(reason) => match guild_id.kick_with_reason(ctx, user.clone(), &reason).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error kicking user: {}", why))),
        },
        Err(_) => match guild_id.kick(ctx, user.clone()).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error kicking user: {}", why))),
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
async fn create_verification(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let channel_id = match args.parse::<ChannelId>() {
        Ok(channel_id) => channel_id,
        Err(why) => {
            return Err(CommandError::from(format!(
                "Invalid channel argument: {}",
                why
            )));
        }
    };

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

#[check]
async fn is_mod(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let config = {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();
        match Config::get_from_db(&pool).await {
            Ok(config) => config,
            Err(why) => {
                return Err(Reason::Log(format!(
                    "Error getting config from database: {}",
                    why
                )))
            }
        }
    };

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
        log::info!("User has moderator role");
        return Ok(());
    }

    Err(Reason::Log("No mod role found".to_string()))
}
