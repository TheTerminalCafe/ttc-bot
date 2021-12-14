use std::collections::HashSet;

use serenity::{
    client::Context,
    framework::standard::{
        help_commands,
        macros::{help, hook},
        Args, CommandError, CommandGroup, CommandResult, DispatchError, HelpOptions,
    },
    model::{channel::Message, id::UserId},
    utils::Color,
};

use crate::utils::helper_functions::embed_msg;

// -----
// Hooks
// -----

#[hook]
pub async fn unknown_command(ctx: &Context, msg: &Message, cmd_name: &str) {
    match embed_msg(
        ctx,
        &msg.channel_id,
        Some("Not a valid command"),
        Some(&format!("No command named {} was found", cmd_name)),
        Some(Color::RED),
        None,
    )
    .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Error sending message: {}", why),
    }
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::NotEnoughArguments { min, given } => {
            match msg
                .channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Not enough arguments")
                            .description(format!(
                                "A minimum of *{}* arguments is required, {} was provided.",
                                min, given
                            ))
                            .color(Color::RED)
                    })
                })
                .await
            {
                Ok(_) => (),
                Err(why) => log::error!("Error sending message: {}", why),
            }
        }
        DispatchError::TooManyArguments { max, given } => {
            match msg
                .channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Too many arguments")
                            .description(format!(
                                "A maximum of *{}* arguments is required, {} was provided.",
                                max, given
                            ))
                            .color(Color::RED)
                    })
                })
                .await
            {
                Ok(_) => (),
                Err(why) => log::error!("Error sending message: {}", why),
            }
        }
        _ => log::warn!("An unhandled dispatch error occurred: {:?}", error),
    }
}

#[hook]
pub async fn after(ctx: &Context, msg: &Message, cmd_name: &str, error: Result<(), CommandError>) {
    match error {
        Ok(_) => (),
        Err(why) => {
            log::error!("Command {} returned with an Err value: {}", cmd_name, why);
            match msg
                .channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("An error occurred")
                            .description(why)
                            .color(Color::RED)
                    })
                })
                .await
            {
                Ok(_) => (),
                Err(why) => {
                    log::error!("Failed to send message: {}", why);
                    return;
                }
            }
        }
    }
}

// Not necessarily a hook but it is close enough so here it shall stay

#[help]
#[embed_error_colour(RED)]
#[embed_success_colour(FOOYOO)]
async fn help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await;
    Ok(())
}
