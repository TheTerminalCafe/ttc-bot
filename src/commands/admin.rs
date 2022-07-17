// --------------------
// Admin group commands
// --------------------

use std::time::Instant;

use poise::serenity_prelude::{
    ButtonStyle, Color, CreateSelectMenu, Emoji, GuildChannel, Role, RoleId,
};
use std::collections::HashMap;

use crate::{
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
    guild_only,
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
    guild_only,
    owners_only,
    hide_in_help,
    category = "Admin"
)]
pub async fn create_selfroles(
    ctx: Context<'_>,
    #[description = "Channel to send it in"] channel: GuildChannel,
) -> Result<(), Error> {
    // Get the channel and guild ids
    let guild_id = ctx.guild_id().unwrap();

    // Create the selection menu
    let mut menu = CreateSelectMenu::default();
    menu.custom_id("ttc-bot-self-role-menu");

    let raw_selfroles = ctx.data().selfroles().await?;

    if raw_selfroles.len() == 0 {
        return Err(Error::from("No roles in the Database"));
    }

    // Set the menu values properly
    menu.min_values(0);
    menu.max_values(raw_selfroles.len() as u64);

    let role_hmap = guild_id.roles(ctx.discord()).await?;
    let emojis = guild_id.emojis(ctx.discord()).await?;

    let mut option_data: Vec<(Role, Option<&Emoji>)> = Vec::new();
    let mut emoji_hmap = HashMap::new();
    for emoji in &emojis {
        emoji_hmap.insert(emoji.name.clone(), emoji.clone());
    }

    for val in raw_selfroles {
        let role = match role_hmap.get(&RoleId(val.0 as u64)) {
            Some(role) => role,
            None => {
                return Err(Error::from(format!("Invalid role with ID {}", val.0)));
            }
        };
        let emoji = emoji_hmap.get(&val.1.unwrap_or(String::from("")));
        option_data.push((role.clone(), emoji));
    }

    // Create the options for the roles
    menu.options(|m| {
        for val in option_data {
            let role = val.0;
            match val.1 {
                Some(emoji) => {
                    m.create_option(|o| o.label(role.name).value(role.id).emoji(emoji.clone()));
                }
                None => {
                    m.create_option(|o| o.label(role.name).value(role.id));
                }
            }
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
    let support_channel = ctx.data().support_channel().await?;
    channel
        .send_message(ctx.discord(), |m| {
            m.embed(|e| {
                e.color(Color::FOOYOO)
                    .title("Support tickets")
                    .description(format!(
                        "{}\n\nAll support tickets are created in <#{}>",
                        description, support_channel
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
    if EmojiCache::is_running() {
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
        let start_time = Instant::now();
        let mut emoji_cache = EmojiCache::new(&ctx.data().pool);
        ctx.send(|b| {
            b.embed(|e| {
                e.title("Starting to rebuild the complete Emoji cache")
                    .description("This is going to take *some* time")
                    .color(Color::FOOYOO)
            })
        })
        .await?;
        emoji_cache.update_emoji_cache_poise(&ctx, true).await?;
        let elapsed = chrono::Duration::from_std(start_time.elapsed())?;
        ctx.send(|b| {
            b.embed(|e| {
                e.title("Finished rebuilding the Emoji cache")
                    .description(format!(
                        "Things should be synced now again, time taken: {}",
                        crate::utils::helper_functions::format_duration(&elapsed)
                    ))
                    .color(Color::FOOYOO)
            })
        })
        .await?;
    }

    Ok(())
}
