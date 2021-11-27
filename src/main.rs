// ----------------------
// Imports from libraries
// ----------------------

use clap::{App, Arg};
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::standard::{
        help_commands,
        macros::{command, group, help, hook},
        Args, CommandError, CommandGroup, CommandResult, HelpOptions, StandardFramework,
    },
    model::{
        channel::Message,
        id::{ChannelId, UserId},
        prelude::{Activity, Ready},
    },
    prelude::TypeMapKey,
    utils::Color,
};
use std::{collections::HashSet, time::Duration};

// --------------------------------------
// Data types to be stored within the bot
// --------------------------------------

struct Threads;
impl TypeMapKey for Threads {
    type Value = Vec<ChannelId>;
}
// --------------
// Command groups
// --------------

#[group]
#[commands(ping)]
struct General;

#[group]
#[prefixes("support")]
#[description("Support related commands")]
#[commands(new, close)]
struct Support;

// ------------
// Help message
// ------------

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

// ----------------
// Helper functions
// ----------------

async fn embed_msg(ctx: &Context, msg: &Message, text: &str, color: Color) -> CommandResult {
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| e.description(text).color(color));
            m
        })
        .await?;
    Ok(())
}

// -------------------------------------
// Event Handler and it's implementation
// -------------------------------------

// Custom handler for events
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::listening("Kirottu's screaming"))
            .await;
    }
}

// -----
// Hooks
// -----

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, cmd_name: &str) {
    if let Err(why) = embed_msg(
        ctx,
        msg,
        &format!("**Error**: No command named: {}", cmd_name),
        Color::RED,
    )
    .await
    {
        println!("An error occurred: {}", why);
    }
}

// ---------------------------------
// Initialization code & Entry point
// ---------------------------------

#[tokio::main]
async fn main() {
    let matches = App::new("TTCBot")
        .arg(
            Arg::with_name("token")
                .takes_value(true)
                .required(true)
                .help("Token to login to discord with")
                .short("t")
                .long("token"),
        )
        .get_matches();

    let token = matches.value_of("token").unwrap();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("ttc!"))
        .help(&HELP)
        .unrecognised_command(unknown_command)
        .group(&GENERAL_GROUP)
        .group(&SUPPORT_GROUP);

    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<Threads>(Vec::new());
    }

    if let Err(why) = client.start().await {
        println!("An error occurred: {}", why);
    }
}

// ----------------------
// General group commands
// ----------------------

#[command]
#[description("Ping!")]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply_ping(ctx, "pong").await?;

    Ok(())
}

// ----------------------
// Support group commands
// ----------------------

#[command]
#[description("Create a new support thread")]
#[only_in(guilds)]
#[min_args(1)]
async fn new(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let thread_name = args.rest();

    embed_msg(ctx, msg, "**Description of issue?**", Color::BLUE).await?;

    let description_msg = match msg
        .author
        .await_reply(ctx)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(msg) => msg,
        None => {
            embed_msg(ctx, msg, "No reply sent in 60 seconds", Color::RED).await?;
            return Err(CommandError::from(
                "No reply received for problem description",
            ));
        }
    };

    embed_msg(
        ctx,
        msg,
        "**System information (OS, OS version, hardware info, other info relevant to issue)**",
        Color::BLUE,
    )
    .await?;

    let system_info_msg = match msg
        .author
        .await_reply(ctx)
        .timeout(Duration::from_secs(60))
        .await
    {
        Some(msg) => msg,
        None => {
            embed_msg(ctx, msg, "No reply sent in 60 seconds", Color::RED).await?;
            return Err(CommandError::from("No reply received for system info"));
        }
    };

    let description = description_msg.content_safe(ctx).await;
    let system_info = system_info_msg.content_safe(ctx).await;

    let thread_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(thread_name)
                    .field("Description of issue:", description, false)
                    .field("System info:", system_info, false)
            })
        })
        .await?;

    let thread_id = msg
        .channel_id
        .create_public_thread(ctx, thread_msg.id, |ct| ct.name(thread_name))
        .await?
        .id;

    let mut data = ctx.data.write().await;
    let threads = match data.get_mut::<Threads>() {
        Some(threads) => threads,
        None => return Err(CommandError::from("No threads vector!")),
    };
    threads.push(thread_id);

    Ok(())
}

#[command]
#[description("Close the current support thread")]
#[only_in(guilds)]
async fn close(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let threads = match data.get_mut::<Threads>() {
        Some(threads) => threads,
        None => return Err(CommandError::from("No threads vector!")),
    };

    if threads.contains(&msg.channel_id) {
        msg.channel_id.delete(ctx).await?;
    } else {
        embed_msg(ctx, msg, "**Error**: Not in a support thread!", Color::RED).await?;
    }

    Ok(())
}
