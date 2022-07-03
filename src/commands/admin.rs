// --------------------
// Admin group commands
// --------------------

use std::collections::HashMap;

use poise::serenity_prelude::{
    ButtonStyle, ChannelType, Color, CreateSelectMenu, GuildChannel, Role, RoleId,
};

use crate::{
    get_config,
    types::{self, Context, Error},
    utils::emoji_cache::EmojiCache,
};

/// Shutdown the bot
///
/// Command to shut down the bot
/// ``shutdown``
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn shutdown(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| m.embed(|e| e.title("Goodbye!").color(Color::PURPLE)))
        .await?;

    ctx.framework()
        .shard_manager
        .lock()
        .await
        .shutdown_all()
        .await;

    Ok(())
}

/// Register slash commands
///
/// Command to register the slash commands
/// ``register``
#[poise::command(prefix_command, owners_only, hide_in_help, category = "Admin")]
pub async fn manage_commands(ctx: types::Context<'_>) -> Result<(), types::Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Create verification button
///
/// Command to create the verification button
/// ``create_verification [channel_id]``
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn create_verification(
    ctx: Context<'_>,
    #[description = "Channel to send it in"] channel: GuildChannel,
) -> Result<(), Error> {
    channel
        .send_message(ctx.discord(), |m| {
            m.embed(|e| e.color(Color::FOOYOO).title("Be sure to follow the rules!"))
                .components(|c| {
                    c.create_action_row(|a| {
                        a.create_button(|b| {
                            b.label("Click here to finish verification")
                                .custom_id("ttc-bot-verification-button")
                                .style(ButtonStyle::Primary)
                        })
                    })
                })
        })
        .await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Verification created")
                .description(format!("Verification prompt created in <#{}>.", channel.id))
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

/// Create selfroles message
///
/// Command to create message for managing permissions
/// ``create_verification [channel_id] [roles (seperated by commas)]``
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn create_selfroles(
    ctx: Context<'_>,
    #[description = "Channel to send it in"] channel: GuildChannel,
    #[description = "List of roles separated by commas"] roles_string: String,
) -> Result<(), Error> {
    // Get the channel and guild ids
    let guild_id = ctx.guild_id().unwrap();

    // Create the selection menu
    let mut menu = CreateSelectMenu::default();
    menu.custom_id("ttc-bot-self-role-menu");

    let raw_role_list = roles_string
        .split(",")
        .map(|role| role.parse::<RoleId>().unwrap_or(RoleId(0)))
        .collect::<Vec<RoleId>>();

    // Create the list for the roles
    let mut role_list: Vec<Role> = Vec::new();

    let roles = guild_id.roles(ctx.discord()).await?;

    // Get the roles
    for role_id in &raw_role_list {
        if roles.contains_key(role_id) {
            let role = roles[&role_id].clone();
            role_list.push(role);
        } else {
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Invalid role")
                        .description("No role with id {} found on this server")
                        .color(Color::RED)
                })
                .ephemeral(true)
            })
            .await?;
        }
    }

    // Make sure some valid roles were procided
    if role_list.len() == 0 {
        return Err(Error::from("None of the provided roles were valid."));
    }

    // Set the menu values properly
    menu.min_values(0);
    menu.max_values(role_list.len() as u64);

    // Create the options for the roles
    menu.options(|m| {
        for role in role_list {
            m.create_option(|o| o.label(role.name).value(role.id));
        }
        m
    });

    // Create the menu in the specified channel
    channel
        .send_message(ctx.discord(), |m| {
            m.components(|c| c.create_action_row(|a| a.add_select_menu(menu)))
                .embed(|e| e.title("Manage your self roles here").color(Color::PURPLE))
        })
        .await?;

    // Reply to the user
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Self-role menu created")
                .description(format!("Self-role menu created in <#{}>.", channel.id))
        })
    })
    .await?;

    Ok(())
}

/// Create support ticket button
///
/// Command to create the button for support tickets
/// ``create_support_ticket_button [channel_id] [description]``
///
/// ``description`` is the description of the embed
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn create_support_ticket_button(
    ctx: Context<'_>,
    #[description = "Channel to send it in"] channel: GuildChannel,
    #[description = "Description for the support system"] description: String,
) -> Result<(), Error> {
    let config = get_config!(ctx.data(), {
        return Err(Error::from("Unable to obtain config"));
    });

    channel
        .send_message(ctx.discord(), |m| {
            m.embed(|e| {
                e.color(Color::FOOYOO)
                    .title("Support tickets")
                    .description(format!(
                        "{}\n\nAll support tickets are created in <#{}>",
                        description, config.support_channel
                    ))
            })
            .components(|c| {
                c.create_action_row(|a| {
                    a.create_button(|b| {
                        b.label("Click here to create a support ticket")
                            .custom_id("ttc-bot-ticket-button")
                            .style(ButtonStyle::Primary)
                    })
                })
            })
        })
        .await?;

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Support button created").description(format!(
                "Support ticket button created in <#{}>",
                channel.id
            ))
        })
    })
    .await?;

    Ok(())
}

/// Create webhooks
///
/// Command to create webhooks for the bot
/// ``create_webhooks [channel_id] [webhook_name]``
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    guild_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn create_webhooks(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Creating webhooks")
                .description("This may take a while, please be patient")
                .color(Color::FOOYOO)
        })
    })
    .await?;
    {
        let mut webhooks = ctx.data().webhooks.write().await;

        for (_, webhook) in webhooks.iter() {
            webhook.delete(ctx.discord()).await?;
            log::info!("Deleted webhook {:?}", webhook.name);
        }
        webhooks.clear();
    }

    let mut webhooks = HashMap::new();

    let channels = ctx.guild_id().unwrap().channels(ctx.discord()).await?;

    log::info!("Creating new webhooks");

    for (channel_id, channel) in &channels {
        if channel.kind == ChannelType::Text {
            let webhook = channel_id
                .create_webhook(
                    ctx.discord(),
                    format!("ttc-bot fancy webhook {}", channel_id),
                )
                .await?;

            log::info!("Created webhook for channel {}", channel_id);

            webhooks.insert(channel_id, webhook);

            ctx.channel_id()
                .send_message(ctx.discord(), |m| {
                    m.content(format!("Webhook created in <#{}>", channel_id))
                })
                .await?;
        }
    }

    log::info!("Clearing the old database table");

    sqlx::query!(r#"DELETE FROM ttc_webhooks"#)
        .execute(&ctx.data().pool)
        .await?;

    let mut webhooks_data = ctx.data().webhooks.write().await;

    log::info!("Updating runtime variables and database");

    for (channel_id, webhook) in &webhooks {
        webhooks_data.insert(**channel_id, webhook.clone());
        sqlx::query!(
            r#"INSERT INTO ttc_webhooks (channel_id, webhook_url) VALUES ($1, $2)"#,
            channel_id.0 as i64,
            match webhook.url() {
                Ok(url) => url,
                Err(why) => {
                    log::error!("Malformed webhook: {}", why);
                    continue;
                }
            }
        )
        .execute(&ctx.data().pool)
        .await?;
    }

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Webhooks created")
                .description("Webhooks created in all text channels")
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

/// Rebuild the Emoji Cache
///
/// Completly rebuild the Emoji cache. This will take some time
/// ``rebuild_emoji_cache``
#[poise::command(
    prefix_command,
    slash_command,
    owners_only,
    guild_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn rebuild_emoji_cache(ctx: Context<'_>) -> Result<(), Error> {
    let emoji_cache = EmojiCache::new_poise(&ctx)?;
    if emoji_cache.is_running() {
        ctx.send(|b| {
            b.embed(|e| {
                e.title("Emoji cache is already being updated")
                    .description("Please try using this command later again")
                    .color(Color::RED)
            })
            .ephemeral(true)
        })
        .await?;
    } else {
        ctx.send(|b| {
            b.embed(|e| {
                e.title("Starting to rebuild the complete Emoji cache")
                    .description("This is going to take *some* time")
                    .color(Color::FOOYOO)
            })
        })
        .await?;
        emoji_cache.update_emoji_cache(true).await?;
        ctx.send(|b| {
            b.embed(|e| {
                e.title("Finished rebuilding the Emoji cache")
                    .description("Things should be synced now again")
                    .color(Color::FOOYOO)
            })
        })
        .await?;
    }

    Ok(())
}
