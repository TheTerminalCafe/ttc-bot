// -------------------
// Module declarations
// -------------------

mod commands {
    pub mod admin;
    //pub mod config;
    pub mod general;
    pub mod localisation;
    pub mod moderation;
    pub mod support;
}
mod utils {
    pub mod autocomplete_functions;
    pub mod helper_functions;
    pub mod macros;
}
mod events {
    pub mod bumpy_business;
    pub mod conveyance;
    pub mod interactions;
    pub mod listener;
    pub mod support;
}
mod types;

// ----------------------
// Imports from libraries
// ----------------------

use clap::{App, Arg};
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures::TryFutureExt;
use poise::serenity_prelude::{Activity, Color, GatewayIntents};
use poise::Command;
use regex::Regex;
use serde_yaml::Value;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook_tokio::Signals;
use sqlx::postgres::PgPoolOptions;
use std::io::Read;
use std::{collections::HashSet, fs::File, sync::Arc};
use types::{Context, Data, Error};

use crate::types::Config;
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

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            log::warn!("Error in command `{}`: {:?}", ctx.command().name, error,);
            match ctx
                .send(|m| {
                    m.embed(|e| {
                        e.title("An error occurred.")
                            .description(format!("{}", error))
                            .color(Color::RED)
                    })
                    .ephemeral(true)
                })
                .await
            {
                Ok(_) => (),
                Err(why) => log::error!("Failed to send error message: {:?}", why),
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                log::error!("Error while handling error: {}", e)
            }
        }
    }
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

    // Write the config to the database if correct argument is present
    if matches.is_present("write-db") {
        let config = Config {
            support_channel: support_channel_id as i64,
            verified_role: verified_role_id as i64,
            moderator_role: moderator_role_id as i64,
            conveyance_channels: conveyance_channel_ids,
            conveyance_blacklisted_channels: conveyance_blacklisted_channel_ids,
            welcome_channel: welcome_channel_id as i64,
            welcome_messages,
        };

        match config.save_in_db(&pool).await {
            Ok(_) => (),
            Err(why) => {
                log::error!("Failed to write config into the database: {}", why);
                return;
            }
        }
    }

    // Create the framework of the bot
    let framework = poise::Framework::build()
        .token(token)
        .client_settings(move |client| client.application_id(application_id))
        .intents(
            GatewayIntents::non_privileged()
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .user_data_setup(move |ctx, ready, _| {
            Box::pin(async move {
                log::info!("Ready! Logged in as {}", ready.user.tag());
                ctx.set_activity(Activity::listening("Kirottu's screaming"))
                    .await;
                Ok(Data {
                    users_currently_questioned: Mutex::new(Vec::new()),
                    pool: pool,
                    thread_name_regex: Regex::new("[^a-zA-Z0-9 ]").unwrap(),
                })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                help(),
                // Admin commands
                commands::moderation::ban(),
                commands::moderation::pardon(),
                commands::admin::register(),
                commands::admin::shutdown(),
                // General commands
                commands::general::ping(),
                commands::general::userinfo(),
                commands::general::harold(),
                // Localisation commands
                commands::localisation::translate(),
                // Moderation commands
                commands::moderation::ban(),
                // Support commands
                commands::support::solve(),
                Command {
                    subcommands: vec![commands::support::title(), commands::support::id()],
                    ..commands::support::search()
                },
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".to_string()),
                ..Default::default()
            },
            owners: owners,
            listener: |ctx, event, framework, data| {
                Box::pin(events::listener::listener(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .build()
        .await
        .unwrap();

    // Handling termination signals gracefully, listen for them and shut down the bot if one is received
    let signals = Signals::new(TERM_SIGNALS).unwrap();
    let handle = signals.handle();

    // Spawn the listening task
    tokio::spawn(signal_hook_task(signals, framework.shard_manager()));

    // Run the bot
    framework.start().await.unwrap();

    // Close the listening task, to make the bot actually shut down
    handle.close();

    log::info!("Bot shut down");
}

async fn signal_hook_task(
    mut signals: Signals,
    shard_mgr: Arc<poise::serenity_prelude::Mutex<poise::serenity_prelude::ShardManager>>,
) {
    while let Some(_) = signals.next().await {
        log::info!("A termination signal received, exiting...");
        shard_mgr.lock().await.shutdown_all().await;
        break;
    }
}
