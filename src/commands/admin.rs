// --------------------
// Admin group commands
// --------------------

use std::time::Instant;

use poise::serenity_prelude::{ButtonStyle, CreateSelectMenu, Emoji, GuildChannel, Role, RoleId};
use std::collections::HashMap;

use crate::{
    traits::context_ext::ContextExt, traits::readable::Readable, utils::emoji_cache::EmojiCache,
    Context, Error,
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
    ctx.send_simple(
        false,
        "Goodbye!",
        None,
        ctx.data().colors.admin_success().await,
    )
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
pub async fn manage_commands(ctx: Context<'_>) -> Result<(), Error> {
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
    let color = ctx.data().colors.verification_message().await;
    channel
        .send_message(ctx.discord(), |m| {
            m.embed(|e| e.color(color).title("Be sure to follow the rules!"))
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

    ctx.send_simple(
        false,
        "Verification created",
        Some(&format!(
            "Verification prompt created in <#{}>.",
            channel.id
        )),
        ctx.data().colors.admin_success().await,
    )
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

    let raw_selfroles = ctx.data().config.selfroles().await?;

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
    let color = ctx.data().colors.selfrole_selection().await;
    channel
        .send_message(ctx.discord(), |m| {
            m.components(|c| c.create_action_row(|a| a.add_select_menu(menu)))
                .embed(|e| e.title("Manage your self roles here").color(color))
        })
        .await?;

    // Reply to the user
    ctx.send_simple(
        false,
        "Self-role menu created",
        Some(&format!("Self-role menu created in <#{}>.", channel.id)),
        ctx.data().colors.admin_success().await,
    )
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
    let support_channel = ctx.data().config.support_channel().await?;
    let color = ctx.data().colors.admin_success().await;
    channel
        .send_message(ctx.discord(), |m| {
            m.embed(|e| {
                e.color(color).title("Support tickets").description(format!(
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

    ctx.send_simple(
        false,
        "Support button created",
        Some(&format!(
            "Support ticket button created in <#{}>",
            channel.id
        )),
        ctx.data().colors.admin_success().await,
    )
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
        ctx.send_simple(
            true,
            "Emoji cache is already being updated",
            Some("Please try using this command later again"),
            ctx.data().colors.input_error().await,
        )
        .await?;
    } else {
        let start_time = Instant::now();
        let mut emoji_cache = EmojiCache::new(&ctx.data().pool);
        ctx.send_simple(
            false,
            "Starting to rebuild the complete Emoji cache",
            Some("This is going to take *some* time"),
            ctx.data().colors.emoji_info().await,
        )
        .await?;
        emoji_cache.update_emoji_cache_poise(&ctx, true).await?;
        ctx.send_simple(
            false,
            "Finished rebuilding the Emoji cache",
            Some(&format!(
                "Things should be synced now again, time taken: {}",
                start_time.elapsed().readable()
            )),
            ctx.data().colors.admin_success().await,
        )
        .await?;
    }

    Ok(())
}
