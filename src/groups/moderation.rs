use crate::utils::helper_functions::embed_msg;
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{channel::Message, id::UserId, interactions::message_component::ButtonStyle},
    utils::Color,
};

#[group]
#[prefix("mod")]
#[allowed_roles("Moderator")]
#[commands(ban, kick, create_verification)]
struct Moderation;

#[command]
#[min_args(1)]
#[max_args(2)]
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
async fn create_verification(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.color(Color::FOOYOO)
                    .description("Be sure to follow the rules!")
            })
            .components(|c| {
                c.create_action_row(|a| {
                    a.create_button(|b| {
                        b.label("Clieck here to finish verification")
                            .custom_id("ttc-bot-verification-button")
                            .style(ButtonStyle::Primary)
                    })
                })
            })
        })
        .await
        .unwrap();

    Ok(())
}
