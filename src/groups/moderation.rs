use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::{channel::Message, id::UserId, prelude::User},
};

#[group]
#[prefixes("mod")]
#[commands(ban)]
struct Moderation;

#[command]
#[min_args(1)]
async fn ban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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

    Ok(())
}
