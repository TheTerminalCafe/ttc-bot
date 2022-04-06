// -------------------
// Module declarations
// -------------------

/*mod typemap {
    pub mod config;
    pub mod types;
}*/
mod groups {
    pub mod admin;
    //    pub mod config;
    pub mod general;
    pub mod localisation;
    pub mod moderation;
    //    pub mod support;
}
mod utils {
    //    pub mod helper_functions;
    pub mod macros;
}
/*mod events {
    pub mod conveyance;
    pub mod interactions;
}*/
mod client {
    //    pub mod event_handler;
    //    pub mod hooks;
}
mod types;

// ----------------------
// Imports from libraries
// ----------------------

use clap::{App, Arg};
use futures::stream::StreamExt;
use poise::serenity_prelude::GatewayIntents;
use regex::Regex;
use serde_yaml::Value;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook_tokio::Signals;
use sqlx::postgres::PgPoolOptions;
use std::io::Read;
use std::{collections::HashSet, fs::File, sync::Arc};
use tokio::sync::Mutex;
use types::{Context, Data, Error};
//use typemap::types::*;
// ------------
// Help message
// ------------

// ---------------------------------
// Initialization code & Entry point
// ---------------------------------

#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "\
This is an example bot made to showcase features of my custom Discord bot framework",
            show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let matches = App::new("TTCBot")
        .arg(
            Arg::with_name("core-config")
                .takes_value(true)
                .required(true)
                .short("c")
                .long("core-config")
                .help("Configuration file"),
        )
        .arg(
            Arg::with_name("write-db")
                .takes_value(false)
                .required(false)
                .short("w")
                .long("write")
                .help("Write the config to the database"),
        )
        .arg(
            Arg::with_name("bad-words")
                .takes_value(true)
                .required(false)
                .short("b")
                .long("bad-words")
                .help("A bad word list, one per line"),
        )
        .arg(
            Arg::with_name("append-bad-words")
                .takes_value(false)
                .required(false)
                .short("a")
                .long("append-bad-words")
                .requires("bad-words")
                .help("Appends provided bad words to the database table"),
        )
        .get_matches();

    // Get environment values from .env for ease of use
    dotenv::dotenv().ok();

    env_logger::init();

    // Load the config file
    let config_file = File::open(matches.value_of("core-config").unwrap()).unwrap();
    let config: Value = serde_yaml::from_reader(config_file).unwrap();

    // Load all the values from the config
    let token = config["token"].as_str().unwrap();
    let application_id = config["application_id"].as_u64().unwrap();
    let sqlx_config = config["sqlx_config"].as_str().unwrap();
    let support_channel_id = config["support_channel"].as_u64().unwrap();
    let verified_role_id = config["verified_role"].as_u64().unwrap();
    let moderator_role_id = config["moderator_role"].as_u64().unwrap();
    let conveyance_channel_ids = config["conveyance_channels"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|val| val.as_i64().unwrap())
        .collect::<Vec<i64>>();
    let conveyance_blacklisted_channel_ids = config["conveyance_blacklisted_channels"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|val| val.as_i64().unwrap())
        .collect::<Vec<i64>>();
    let welcome_channel_id = config["welcome_channel"].as_u64().unwrap();
    let welcome_messages = config["welcome_messages"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|val| val.as_str().unwrap().to_string())
        .collect::<Vec<String>>();
    let mut owners = HashSet::new();

    for owner in config["owners"].as_sequence().unwrap() {
        owners.insert(poise::serenity_prelude::UserId(owner.as_u64().unwrap()));
    }

    // Create the connection to the database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(sqlx_config)
        .await
        .unwrap();

    /*if matches.is_present("additional-config") {
        let config = typemap::config::Config {
            support_channel: support_channel_id as i64,
            conveyance_channels: conveyance_channel_ids,
            conveyance_blacklisted_channels: conveyance_blacklisted_channel_ids,
            welcome_channel: welcome_channel_id as i64,
            verified_role: verified_role_id as i64,
            moderator_role: moderator_role_id as i64,
            welcome_messages,
        };

        config.save_in_db(&pool).await.unwrap();
    }*/

    if matches.is_present("bad-words") {
        let mut file = File::open(matches.value_of("bad-words").unwrap()).unwrap();
        let mut raw_string = String::new();
        file.read_to_string(&mut raw_string).unwrap();

        if !matches.is_present("append-bad-words") {
            match sqlx::query!(r#"DELETE FROM ttc_bad_words"#)
                .execute(&pool)
                .await
            {
                Ok(_) => (),
                Err(why) => {
                    log::error!("Failed to clear bad word database: {}", why);
                    return;
                }
            }
        }
        for line in raw_string.lines() {
            let line = line.trim();
            match sqlx::query!(r#"INSERT INTO ttc_bad_words (word) VALUES($1)"#, line)
                .execute(&pool)
                .await
            {
                Ok(_) => (),
                Err(why) => {
                    log::error!("Failed to write bad words into the database: {}", why);
                    return;
                }
            }
        }
    }
    /*
    // Create the framework of the bot
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("ttc!").owners(owners))
        .help(&client::hooks::HELP)
        .unrecognised_command(client::hooks::unknown_command)
        .on_dispatch_error(client::hooks::dispatch_error)
        .after(client::hooks::after)
        .group(&groups::general::GENERAL_GROUP)
        .group(&groups::support::SUPPORT_GROUP)
        .group(&groups::admin::ADMIN_GROUP)
        .group(&groups::config::CONFIG_GROUP)
        .group(&groups::moderation::MODERATION_GROUP)
        .group(&groups::localisation::LOCALISATION_GROUP);

    // Create the bot client
    let mut client = Client::builder(token)
        .application_id(application_id)
        .event_handler(client::event_handler::Handler)
        .cache_settings(|c| c.max_messages(50))
        .framework(framework)
        .intents(GatewayIntents::non_privileged() | GatewayIntents::GUILD_MEMBERS)
        .await
        .expect("Error creating client");

    // Initial data for the bot
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerType>(client.shard_manager.clone());
        data.insert::<ThreadNameRegexType>("Regex::new("[^a-zA-Z0-9 ]").unwrap()");
        data.insert::<UsersCurrentlyQuestionedType>(Vec::new());
        data.insert::<PgPoolType>(pool);
    }


    match client.start().await {
        Ok(_) => (),
        Err(why) => log::error!("An error occurred when starting the client: {}", why),
    }*/
    /*let signals = match Signals::new(TERM_SIGNALS) {
        Ok(signals) => signals,
        Err(why) => {
            log::error!("Failed to create signal hook: {}", why);
            return;
        }
    };*/

    //let handle = signals.handle();

    //tokio::spawn(signal_hook_task(signals, client.shard_manager.clone()));

    log::info!("Got here");

    poise::Framework::build()
        .token(token)
        .client_settings(move |client| {
            client.application_id(application_id).intents(
                GatewayIntents::non_privileged()
                    | GatewayIntents::GUILD_MEMBERS
                    | GatewayIntents::MESSAGE_CONTENT,
            )
        })
        .user_data_setup(move |ctx, ready, framework| {
            Box::pin(async move {
                log::info!("Ready I guess?");
                log::info!("{:?}", ready);
                Ok(Data {
                    users_currently_questioned: Vec::new(),
                    pool: pool,
                    thread_name_regex: Regex::new("[^a-zA-Z0-9 ]").unwrap(),
                })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                help(),
                groups::admin::register(),
                groups::general::ping(),
                groups::localisation::translate(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".to_string()),
                ..Default::default()
            },
            owners: owners,
            ..Default::default()
        })
        .run()
        .await
        .unwrap();

    //handle.close();

    log::info!("Bot shut down");
}

async fn signal_hook_task(
    mut signals: Signals,
    shard_mgr: Arc<Mutex<poise::serenity_prelude::ShardManager>>,
) {
    while let Some(_) = signals.next().await {
        log::info!("A termination signal received, exiting...");
        shard_mgr.lock().await.shutdown_all().await;
        break;
    }
}
