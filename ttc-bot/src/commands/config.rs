use poise::serenity_prelude::{Color, Message};

use crate::{
    command_error, get_config, utils::helper_functions::embed_msg, types::Context,
};

#[poise::command(slash_command, prefix_command, owner_only)]
async fn set(ctx: Context<'_>, #[description = "The name of the value"] property: String, #[description = "Value to set it to"]) -> CommandResult {
    args.quoted();
    // Get the config from the database
    let mut config = get_config!(ctx, { return command_error!("Database error.") });

    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();
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
        "conveyance_channels" => {
            let mut values: Vec<i64> = Vec::new();
            for value in args.iter() {
                let value: String = value?;
                values.push(match value.parse::<i64>() {
                    Ok(value) => value,
                    Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
                })
            }
            config.conveyance_channels = values;
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
        "verified_role" => {
            let value: String = args.single()?;
            config.verified_role = match value.parse::<i64>() {
                Ok(value) => value,
                Err(why) => return Err(CommandError::from(format!("Parsing error: {}", why))),
            }
        }
        "moderator_role" => {
            let value: String = args.single()?;
            config.moderator_role = match value.parse::<i64>() {
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
    let config = get_config!(ctx, { return command_error!("Database error.") });

    let property: String = args.single()?;

    // Match the requested config field to predefined fields
    let property_value = match &property[..] {
        "support_channel" => format!("{}", config.support_channel),
        "conveyance_channel" => format!("{:?}", config.conveyance_channels),
        "conveyance_blacklisted_channels" => {
            format!("{:?}", config.conveyance_blacklisted_channels)
        }
        "welcome_channel" => format!("{}", config.welcome_channel),
        "verified_role" => format!("{}", config.verified_role),
        "moderator_role" => format!("{}", config.moderator_role),
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
