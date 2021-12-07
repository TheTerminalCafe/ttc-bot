// -------------------
// Module declarations
// -------------------

mod admin;
mod conveyance;
mod general;
mod helper_functions;
mod support;

// ----------------------
// Imports from libraries
// ----------------------

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
        channel::{GuildChannel, Message},
        event::MessageUpdateEvent,
        id::{ChannelId, GuildId, MessageId, UserId},
        misc::Mentionable,
        prelude::{Activity, Ready},
    },
    prelude::{Mutex, TypeMapKey},
    utils::Color,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashSet, fs::File, sync::Arc, time::Duration};
use support::SupportThread;

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

struct ConveyanceChannelType;
impl TypeMapKey for ConveyanceChannelType {
    type Value = u64;
}

struct BoostLevelType;
impl TypeMapKey for BoostLevelType {
    type Value = u64;
}

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
            match msg.channel_id.say(ctx, "*hides*").await {
                Ok(_) => (),
                Err(why) => println!("Error sending message: {}", why),
            }
        }
    }

    // Update thread status on the database when it is updated
    async fn thread_update(&self, ctx: Context, thread: GuildChannel) {
        // Make sure the updated part is the archived value
        if thread.thread_metadata.unwrap().archived {
            let data = ctx.data.read().await;
            let pool = data.get::<PgPoolType>().unwrap();

            // Get the current thread info from the database
            let db_thread = match sqlx::query_as!(
                SupportThread,
                r#"SELECT * FROM ttc_support_tickets WHERE thread_id = $1"#,
                thread.id.0 as i64
            )
            .fetch_one(pool)
            .await
            {
                Ok(thread) => thread,
                Err(_) => return,
            };

            // Make sure the thread isn't marked as solved
            if !db_thread.incident_solved {
                match thread.edit_thread(&ctx, |t| t.archived(false)).await {
                    Ok(_) => (),
                    Err(why) => {
                        println!("Thread unarchival failed: {}", why);
                        return;
                    }
                }
                // Inform the author of the issue about the unarchival
                match thread
                    .id
                    .send_message(&ctx, |c| {
                        c.content(format!("{}", UserId(db_thread.user_id as u64).mention())).embed(|e| {
                            e.description("If the issue has already been solved make sure to mark it as such with `ttc!support solve`")
                                .title("Thread unarchived")})
                    })
                    .await {
                    Ok(_) => (),
                    Err(why) => println!("Failed to send message: {}", why),
                }
            }
        }
    }
    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        _: Option<GuildId>,
    ) {
        conveyance::message_delete(&ctx, &channel_id, &deleted_message_id).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        conveyance::message_update(&ctx, old_if_available, &event).await;
    }
}

// -----
// Hooks
// -----

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, cmd_name: &str) {
    match embed_msg(
        ctx,
        &msg.channel_id,
        &format!("**Error**: No command named: {}", cmd_name),
        Color::RED,
        false,
        Duration::from_secs(0),
    )
    .await
    {
        Ok(_) => (),
        Err(why) => println!("An error occurred: {}", why),
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
    let support_channel_id = config["support_channel"].as_u64().unwrap();
    let conveyance_channel_id = config["conveyance_channel"].as_u64().unwrap();
    let boost_level = config["boost_level"].as_u64().unwrap(); // For selecting default archival period
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
        .cache_settings(|c| c.max_messages(50))
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
        data.insert::<SupportChannelType>(support_channel_id);
        data.insert::<ConveyanceChannelType>(conveyance_channel_id);
        data.insert::<BoostLevelType>(boost_level);
    }

    match client.start().await {
        Ok(_) => (),
        Err(why) => println!("An error occurred: {}", why),
    }

    println!("goodbye");
}
