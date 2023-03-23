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
    pub mod bee_script;
    pub mod bee_utils;
    pub mod emoji_cache;
    pub mod helper_functions;
    pub mod macros;
    pub mod userinfo;
}
mod events {
    pub mod bee;
    pub mod bumpy_business;
    pub mod conveyance;
    pub mod easter_egg;
    pub mod emoji_cache;
    pub mod interactions;
    pub mod listener;
    pub mod support;
}
mod types {
    pub mod colors;
    pub mod config;
    pub mod data;
}
mod traits {
    pub mod context_ext;
    pub mod readable;
}

// ----------------------
// Imports from libraries
// ----------------------

use clap::{Arg, Command};
use futures::stream::StreamExt;
use poise::serenity_prelude::{Activity, ChannelId, GatewayIntents, RwLock};
use regex::Regex;
use serde_yaml::Value;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook_tokio::Signals;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::io::Read;
use std::time::Instant;
use std::{collections::HashSet, fs::File, sync::Arc};
use types::{colors::Colors, config::Config, data::Data};

// Context and error types to be used in the crate
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    let (ctx, title, description) = match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            log::warn!("Error in command `{}`: {:?}", ctx.command().name, error,);
            (
                ctx,
                "An error occurred",
                format!(
                    "An error occurred in command `{}`: {}, user: {}",
                    ctx.command().name,
                    error,
                    ctx.author().tag()
                ),
            )
        }
        poise::FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
        } => {
            log::warn!(
                "Missing permissions for command `{}`: {:?}, user: {}",
                ctx.command().name,
                missing_permissions,
                ctx.author().tag()
            );
            (
                ctx,
                "Missing permissions",
                match missing_permissions {
                    Some(permissions) => format!(
                        "You are missing the following permissions for command `{}`: {}",
                        ctx.command().name,
                        permissions
                    ),
                    None => format!("You may be missing permissions for command `{}`. Not executing for safety.", ctx.command().name),
            })
        }
        poise::FrameworkError::NotAnOwner { ctx } => {
            log::warn!(
                "User `{}` is not an owner, command called: `{}`",
                ctx.author().tag(),
                ctx.command().name
            );
            (
                ctx,
                "Not an owner",
                format!("This command is for owners only."),
            )
        }
        poise::FrameworkError::ArgumentParse { error, input, ctx } => {
            log::warn!(
                "Error parsing arguments for command `{}`: {:?}, input: {:?}, user: {}",
                ctx.command().name,
                error,
                input,
                ctx.author().tag()
            );
            (
                ctx,
                "Error parsing arguments",
                format!(
                    "Error parsing arguments for command `{}`: {}, input: {:?}, user: {}",
                    ctx.command().name,
                    error,
                    input,
                    ctx.author().tag()
                ),
            )
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx } => {
            log::warn!(
                "Command check failed for command `{}`: {:?}, user: {}",
                ctx.command().name,
                error,
                ctx.author().tag()
            );
            (
                ctx,
                "Command check failed",
                format!(
                    "Command check failed for command `{}`: {:?}, user: {}",
                    ctx.command().name,
                    error,
                    ctx.author().tag()
                ),
            )
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                log::error!("Error while handling error: {}", e)
            }
            return;
        }
    };

    let color = ctx.data().colors.general_error().await;
    match ctx
        .send(|m| {
            m.embed(|e| e.title(title).description(description).color(color))
                .ephemeral(true)
        })
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Error sending error reply message: {}", why),
    }
}

#[tokio::main]
async fn main() {
    let matches = Command::new("TTCBot")
        .arg(
            Arg::new("core-config")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .required(true)
                .short('c')
                .long("core-config")
                .help("Configuration file"),
        )
        .arg(
            Arg::new("bad-words")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .required(false)
                .short('b')
                .long("bad-words")
                .help("A bad word list, one per line"),
        )
        .arg(
            Arg::new("append-bad-words")
                .action(clap::ArgAction::SetTrue)
                .required(false)
                .short('a')
                .long("append-bad-words")
                .requires("bad-words")
                .help("Appends provided bad words to the database table"),
        )
        .get_matches();

    env_logger::init();

    // Load the config file
    let config_file = File::open(matches.get_one::<String>("core-config").unwrap()).unwrap();
    let config: Value = serde_yaml::from_reader(config_file).unwrap();

    // Load all the values from the config
    let token = config["token"].as_str().unwrap();
    let application_id = config["application_id"].as_u64().unwrap();
    let sqlx_config = config["sqlx_config"].as_str().unwrap();
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

    if matches.contains_id("bad-words") {
        let mut file = File::open(matches.get_one::<String>("bad-words").unwrap()).unwrap();
        let mut raw_string = String::new();
        file.read_to_string(&mut raw_string).unwrap();

        if !matches.get_flag("append-bad-words") {
            unwrap_or_return!(
                sqlx::query!(r#"DELETE FROM ttc_bad_words"#)
                    .execute(&pool)
                    .await,
                "Failed to clear bad word database"
            );
        }
        for line in raw_string.lines() {
            let line = line.trim();
            unwrap_or_return!(
                sqlx::query!(r#"INSERT INTO ttc_bad_words (word) VALUES($1)"#, line)
                    .execute(&pool)
                    .await,
                "Failed to write bad words into the database"
            );
        }
    }

    // Create the framework of the bot
    let framework = poise::Framework::builder()
        .token(token)
        .client_settings(move |client| client.application_id(application_id))
        .intents(
            GatewayIntents::non_privileged()
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .setup(move |ctx, ready, _| {
            Box::pin(async move {
                log::info!("Ready! Logged in as {}", ready.user.tag());
                ctx.set_activity(Activity::listening("Kirottu's screaming"))
                    .await;

                let query = sqlx::query!(r#"SELECT * FROM ttc_webhooks"#)
                    .fetch_all(&pool)
                    .await?;

                let mut webhooks = HashMap::new();

                for record in &query {
                    webhooks.insert(
                        ChannelId(record.channel_id as u64),
                        ctx.http.get_webhook_from_url(&record.webhook_url).await?,
                    );
                }

                let pool = Arc::new(pool);
                let config = Config::new(Arc::clone(&pool));
                let colors = Colors::new(Arc::clone(&pool));

                Ok(Data {
                    harold_message: RwLock::new(None),
                    beeified_users: RwLock::new(HashMap::new()),
                    beezone_channels: RwLock::new(HashMap::new()),
                    webhooks: RwLock::new(webhooks),
                    pool: pool,
                    thread_name_regex: Regex::new("[^a-zA-Z0-9 ]").unwrap(),
                    startup_time: Instant::now(),
                    config: config,
                    colors: colors,
                })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                // Admin commands
                commands::admin::manage_commands(),
                commands::admin::shutdown(),
                commands::admin::create_verification(),
                commands::admin::create_selfroles(),
                commands::admin::create_support_ticket_button(),
                commands::admin::rebuild_emoji_cache(),
                // General commands
                commands::general::ping(),
                commands::general::version(),
                commands::general::userinfo(),
                commands::general::userinfo_ctxmenu(),
                commands::general::serverinfo(),
                commands::general::leaderboard(),
                commands::general::help(),
                // Localisation commands
                commands::localisation::translate(),
                commands::localisation::translate_to_en(),
                // Moderation commands
                commands::moderation::purge(),
                commands::moderation::mute(),
                commands::moderation::unmute(),
                commands::moderation::kick(),
                commands::moderation::ban(),
                commands::moderation::pardon(),
                commands::moderation::beeify(),
                commands::moderation::unbeeify(),
                commands::moderation::beezone(),
                commands::moderation::unbeezone(),
                commands::moderation::idban(),
                // Support commands
                commands::support::solve(),
                commands::support::search(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("ttc!".to_string()),
                ..Default::default()
            },
            owners: owners,
            event_handler: |ctx, event, framework, data| {
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
    // tokio::spawn(signal_hook_task(signals, framework.shard_manager())); // TODO: Reimplement
    // this

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
        (*shard_mgr).lock().await.shutdown_all().await;
        break;
    }
}
