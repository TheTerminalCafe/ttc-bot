// -------------------
// Module declarations
// -------------------

mod typemap {
    pub mod config;
    pub mod types;
}
mod groups {
    pub mod admin;
    pub mod config;
    pub mod general;
    pub mod moderation;
    pub mod support;
}
mod utils {
    pub mod helper_functions;
}
mod logging {
    pub mod conveyance;
}
mod client {
    pub mod event_handler;
    pub mod hooks;
}

// ----------------------
// Imports from libraries
// ----------------------

use clap::{App, Arg};
use regex::Regex;
use serde_yaml::Value;
use serenity::{
    client::{bridge::gateway::GatewayIntents, Client},
    framework::standard::StandardFramework,
    model::id::UserId,
};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashSet, fs::File};
use typemap::types::*;

// ------------
// Help message
// ------------

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
        .arg(
            Arg::with_name("write-db")
                .takes_value(false)
                .required(false)
                .short("w")
                .long("write"),
        )
        .get_matches();

    // Get environment values from .env for ease of use
    dotenv::dotenv().ok();

    env_logger::init();

    // Load the config file
    let config_file = File::open(matches.value_of("config").unwrap()).unwrap();
    let config: Value = serde_yaml::from_reader(config_file).unwrap();

    // Load all the values from the config
    let token = config["token"].as_str().unwrap();
    let sqlx_config = config["sqlx_config"].as_str().unwrap();
    let support_channel_id = config["support_channel"].as_u64().unwrap();
    let conveyance_channel_id = config["conveyance_channel"].as_u64().unwrap();
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
        owners.insert(UserId(owner.as_u64().unwrap()));
    }

    // Create the connection to the database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(sqlx_config)
        .await
        .unwrap();

    if matches.is_present("write-db") {
        let config = typemap::config::Config {
            support_channel: support_channel_id as i64,
            conveyance_channel: conveyance_channel_id as i64,
            conveyance_blacklisted_channels: conveyance_blacklisted_channel_ids,
            welcome_channel: welcome_channel_id as i64,
            welcome_messages,
        };

        config.save_in_db(&pool).await.unwrap();
    }

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
        .group(&groups::moderation::MODERATION_GROUP);

    // Create the bot client
    let mut client = Client::builder(token)
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
        data.insert::<ThreadNameRegexType>(Regex::new("[^a-zA-Z0-9 ]").unwrap());
        data.insert::<UsersCurrentlyQuestionedType>(Vec::new());
        data.insert::<PgPoolType>(pool);
    }

    match client.start().await {
        Ok(_) => (),
        Err(why) => log::error!("An error occurred when starting the client: {}", why),
    }

    log::info!("Bot shut down");
}
