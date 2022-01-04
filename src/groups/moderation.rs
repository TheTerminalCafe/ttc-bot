use crate::utils::helper_functions::embed_msg;
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{channel::Message, id::UserId},
    utils::Color,
};

#[group]
#[prefix("mod")]
#[allowed_roles("Moderator")]
#[commands(ban)]
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
        Ok(reason) => match guild_id.ban_with_reason(ctx, user, 2, reason).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error banning user: {}", why))),
        },
        Err(_) => match guild_id.ban(ctx, user, 2).await {
            Ok(_) => (),
            Err(why) => return Err(CommandError::from(format!("Error banning user: {}", why))),
        },
    };

    Ok(())
}
