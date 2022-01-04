use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::channel::Message,
    utils::Color,
};

use crate::{
    typemap::{config, types::PgPoolType},
    utils::helper_functions::embed_msg,
};

#[group]
#[owners_only]
#[prefix("config")]
#[commands(set, get)]
struct Config;

#[command]
#[min_args(2)]
async fn set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    args.quoted();

    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();
    // Get the config from the database
    let mut config = match config::Config::get_from_db(pool).await {
        Ok(config) => config,
        Err(why) => {
            return Err(CommandError::from(format!(
                "Error reading from the database: {}",
                why
            )));
        }
    };
    let property: String = args.single()?;

    // Match the requested config field to predefined fields
    match &property[..] {
        "support_channel" => {
            let value: String = args.single()?;
            config.support_channel = match value.parse::<i64>() {
                Ok(value) => value,
                Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
            }
        }
        "conveyance_channel" => {
            let value: String = args.single()?;
            config.conveyance_channel = match value.parse::<i64>() {
                Ok(value) => value,
                Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
            }
        }
        "conveyance_blacklisted_channels" => {
            let mut values: Vec<i64> = Vec::new();
            for value in args.iter() {
                let value: String = value?;
                values.push(match value.parse::<i64>() {
                    Ok(value) => value,
                    Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
                })
            }
            config.conveyance_blacklisted_channels = values;
        }
        "welcome_channel" => {
            let value: String = args.single()?;
            config.welcome_channel = match value.parse::<i64>() {
                Ok(value) => value,
                Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
            }
        }
        "welcome_messages" => {
            let mut values: Vec<String> = Vec::new();
            for value in args.iter() {
                let value: String = value?;
                values.push(value[1..value.len() - 1].to_string());
            }
            config.welcome_messages = values;
        }
        _ => {
            embed_msg(
                ctx,
                &msg.channel_id,
                Some("Nothing found"),
                Some(&format!("No field found for name {}.", property)),
                Some(Color::RED),
                None,
            )
            .await?;
            return Ok(());
        }
    };

    match config.save_in_db(pool).await {
        Ok(_) => (),
        Err(why) => {
            return Err(CommandError::from(format!(
                "Error saving config into database: {}",
                why
            )))
        }
    }

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Value set successfully"),
        Some(&format!("Value for `{}` set successfully", property)),
        Some(Color::PURPLE),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[num_args(1)]
async fn get(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    // Get the config from the database
    let config = match config::Config::get_from_db(pool).await {
        Ok(config) => config,
        Err(why) => {
            return Err(CommandError::from(format!(
                "Error reading from the database: {}",
                why
            )));
        }
    };
    let property: String = args.single()?;

    // Match the requested config field to predefined fields
    let property_value = match &property[..] {
        "support_channel" => format!("{}", config.support_channel),
        "conveyance_channel" => format!("{}", config.conveyance_channel),
        "conveyance_blacklisted_channels" => {
            format!("{:?}", config.conveyance_blacklisted_channels)
        }
        "welcome_channel" => format!("{}", config.welcome_channel),
        "welcome_messages" => format!("{:?}", config.welcome_messages),
        _ => {
            embed_msg(
                ctx,
                &msg.channel_id,
                Some("Nothing found"),
                Some(&format!("No field found for name {}.", property)),
                Some(Color::RED),
                None,
            )
            .await?;
            return Ok(());
        }
    };

    embed_msg(
        ctx,
        &msg.channel_id,
        Some(&format!("Value for `{}`", property)),
        Some(&property_value),
        Some(Color::PURPLE),
        None,
    )
    .await?;

    Ok(())
}
