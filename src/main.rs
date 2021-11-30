// ----------------------
// Imports from libraries
// ----------------------

mod admin;
mod general;
mod helper_functions;
mod support;

use clap::{App, Arg};
use helper_functions::embed_msg;
use regex::Regex;
use serde_yaml::Value;
use serenity::{
    async_trait,
    client::{bridge::gateway::ShardManager, Client, Context, EventHandler},
    framework::standard::{
        help_commands,
        macros::{help, hook},
        Args, CommandGroup, CommandResult, DispatchError, HelpOptions, StandardFramework,
    },
    model::{
        channel::Message,
        id::UserId,
        prelude::{Activity, Ready},
    },
    prelude::{Mutex, TypeMapKey},
    utils::Color,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashSet, fs::File, sync::Arc};

// --------------------------------------
// Data types to be stored within the bot
// --------------------------------------

struct ShardManagerType;
impl TypeMapKey for ShardManagerType {
    type Value = Arc<Mutex<ShardManager>>;
}

struct ThreadNameRegexType;
impl TypeMapKey for ThreadNameRegexType {
    type Value = Regex;
}

struct UsersCurrentlyQuestionedType;
impl TypeMapKey for UsersCurrentlyQuestionedType {
    type Value = Vec<UserId>;
}

struct PgPoolType;
impl TypeMapKey for PgPoolType {
    type Value = PgPool;
}

struct SupportChannelType;
impl TypeMapKey for SupportChannelType {
    type Value = u64;
}

// --------------
// Command groups
// --------------
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

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.contains("bots will take over the world") {
            if let Err(why) = msg.channel_id.say(ctx, "*hides*").await {
                println!("Error sending message: {}", why);
            }
        }
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

#[hook]
async fn dispatch_error(_: &Context, _: &Message, error: DispatchError) {
    println!("An error occurred: {:?}", error);
}

// ---------------------------------
// Initialization code & Entry point
// ---------------------------------

#[tokio::main]
async fn main() {
    let matches = App::new("TTCBot")
        .arg(
            Arg::with_name("config")
                .takes_value(true)
                .required(true)
                .short("c")
                .long("config"),
        )
        .get_matches();

    // Get environment values from .env for ease of use
    dotenv::dotenv().ok();

    // Load the config file
    let config_file = File::open(matches.value_of("config").unwrap()).unwrap();
    let config: Value = serde_yaml::from_reader(config_file).unwrap();

    // Load all the values from the config
    let token = config["token"].as_str().unwrap();
    let sqlx_config = config["sqlx_config"].as_str().unwrap();
    let support_chanel_id = config["support_channel"].as_u64().unwrap();
    let mut owners = HashSet::new();

    for owner in config["owners"].as_sequence().unwrap() {
        owners.insert(UserId(owner.as_u64().unwrap()));
    }

    // Create the connection to the database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(sqlx_config)
        .await
        .unwrap();

    // Create the framework of the bot
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("ttc!").owners(owners))
        .help(&HELP)
        .unrecognised_command(unknown_command)
        .on_dispatch_error(dispatch_error)
        .group(&general::GENERAL_GROUP)
        .group(&support::SUPPORT_GROUP)
        .group(&admin::ADMIN_GROUP);

    // Create the bot client
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // Initial data for the bot
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerType>(client.shard_manager.clone());
        data.insert::<ThreadNameRegexType>(Regex::new("[^a-zA-Z0-9 ]").unwrap());
        data.insert::<UsersCurrentlyQuestionedType>(Vec::new());
        data.insert::<PgPoolType>(pool);
        data.insert::<SupportChannelType>(support_chanel_id);
    }

    if let Err(why) = client.start().await {
        println!("An error occurred: {}", why);
    }

    println!("goodbye");
}
