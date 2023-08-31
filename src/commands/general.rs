use crate::{
    traits::context_ext::ContextExt, traits::readable::Readable, types::data::Data,
    utils::emoji_cache::EmojiCache, utils::userinfo, utils::userinfo::userinfo_fn, Context, Error,
};
use futures::StreamExt;
use poise::{
    serenity_prelude::{CreateEmbed, Member, User},
    Command,
};
use std::{collections::HashMap, iter::Iterator, time::Duration};
// ----------------------
// General group commands
// ----------------------

/// Ping command
///
/// Command that the bot will respond to with some statistics
/// ``ping``
#[poise::command(prefix_command, slash_command, category = "General")]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let mut embed = CreateEmbed::default();
    let color = ctx.data().colors.ping().await;

    embed.title("Pong!").color(color).field(
        "Uptime",
        ctx.data().startup_time.elapsed().readable(),
        false,
    );
    let message = ctx
        .send(|m| {
            m.embed(|e| {
                e.clone_from(&embed);
                e
            })
        })
        .await?;

    let received_response = message.message().await?.id.created_at();
    let ping = received_response.timestamp_millis() - ctx.created_at().timestamp_millis();

    message
        .edit(ctx, |e| {
            e.embed(|e| {
                e.clone_from(&embed);
                e.field("Ping", format!("{}ms", ping), false)
            })
        })
        .await?;

    Ok(())
}

#[poise::command(context_menu_command = "User info", category = "General", hide_in_help)]
pub async fn userinfo_ctxmenu(ctx: Context<'_>, user: User) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let reply = userinfo_fn(ctx, user, None, false).await?;
    if reply.is_none() {
        return Ok(());
    }
    let reply = reply.unwrap();
    ctx.send(|m| {
        m.clone_from(&reply);
        m
    })
    .await?;
    Ok(())
}

/// User info for a user
///
/// Can be used to get the user info of the specified user. A context menu command is also available
/// ``userinfo [user] [emoji_stats]``
#[poise::command(prefix_command, slash_command, category = "General")]
pub async fn userinfo(
    ctx: Context<'_>,
    #[description = "User"] user: Option<User>,
    #[description = "Emoji stats"] emoji_stats: Option<bool>,
    #[description = "Update Emoji stats before"] update_emojis: Option<bool>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let mut img_path = None;
    let path;
    if emoji_stats.unwrap_or(false) {
        path = userinfo::get_image_output_path()?;
        img_path = Some(path.as_str());
    }
    let reply = userinfo_fn(
        ctx,
        user.unwrap_or(ctx.author().clone()),
        img_path,
        update_emojis.unwrap_or(false),
    )
    .await?;
    if reply.is_none() {
        return Ok(());
    }
    let reply = reply.unwrap();
    ctx.send(|m| {
        m.clone_from(&reply);
        m
    })
    .await?;

    // Lock needs to be released *after* the message was sent
    // acquisition is done in userinfo_fn()
    if img_path.is_some() {
        userinfo::IS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

/// Server info
///
/// Can be used to get server info
/// ``serverinfo``
#[poise::command(prefix_command, slash_command, guild_only, category = "General")]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let color = ctx.data().colors.user_server_info().await;
    let guild = ctx.guild().unwrap();
    let guild_id_part = guild.id.to_partial_guild_with_counts(ctx).await?;
    let online_members = match guild_id_part.approximate_presence_count {
        Some(s) => s.to_string(),
        None => String::from("N/A"),
    };
    let icon = guild.icon_url().unwrap_or(String::from("N/A"));
    let emojis = guild.emojis(ctx).await?;
    let mut fields: Vec<(String, String, bool)> = Vec::new();

    fields.push(("Guild name".to_string(), guild.name.clone(), false));
    fields.push((
        "Server owner".to_string(),
        format!("<@{}>", guild.owner_id.0),
        false,
    ));
    fields.push((
        "Online Members".to_string(),
        format!("{}/{}", online_members, guild.member_count),
        false,
    ));

    let mut tmp = String::new();
    let mut count = 1;
    emojis
        .iter()
        .filter(|emoji| !(emoji.animated))
        .for_each(|emoji| {
            let t = format!("{}<:{}:{}> ", tmp, emoji.name, emoji.id.0);
            if t.len() > 1024 {
                fields.push((
                    format!("Custom Emojis {}", count).to_string(),
                    tmp.clone(),
                    false,
                ));
                tmp = format!("<:{}:{}> ", emoji.name, emoji.id.0);
                count += 1;
            } else {
                tmp = t;
            }
        });
    if count > 1 {
        fields.push((
            format!("Custom Emojis {}", count).to_string(),
            tmp.clone(),
            false,
        ));
    } else {
        fields.push(("Custom Emojis".to_string(), tmp.clone(), false));
    }

    tmp = String::new();
    count = 1;
    emojis
        .iter()
        .filter(|emoji| emoji.animated)
        .for_each(|emoji| {
            let t = format!("{}<a:{}:{}> ", tmp, emoji.name, emoji.id.0);
            if t.len() > 1024 {
                fields.push((
                    format!("Animated Emojis {}", count).to_string(),
                    tmp.clone(),
                    false,
                ));
                tmp = format!("<a:{}:{}> ", emoji.name, emoji.id.0);
                count += 1;
            } else {
                tmp = t;
            }
        });
    if count > 1 {
        fields.push((
            format!("Animated Emojis {}", count).to_string(),
            tmp.clone(),
            false,
        ));
    } else {
        fields.push(("Animated Emojis".to_string(), tmp.clone(), false));
    }

    tmp = String::new();
    count = 1;
    guild
        .roles
        .iter()
        .filter(|role| role.1.name != "@everyone")
        .for_each(|role| {
            let t = format!("{}<@&{}> ", tmp, role.0 .0);
            if t.len() > 1024 {
                fields.push((format!("Roles {}", count).to_string(), tmp.clone(), false));
                tmp = format!("{}<@&{}> ", tmp, role.0 .0);
                count += 1;
            } else {
                tmp = t;
            }
        });
    if count > 1 {
        fields.push((format!("Roles {}", count).to_string(), tmp.clone(), false));
    } else {
        fields.push(("Roles".to_string(), tmp.clone(), false));
    }

    fields.push(("Icon URL".to_string(), icon.clone(), false));

    ctx.send(|m| {
        m.embed(|e| {
            e.author(|a| a.name(&guild.name))
                .fields(fields)
                .color(color)
                .thumbnail(&icon)
        })
        .ephemeral(true)
    })
    .await?;

    Ok(())
}

/// Leaderboards
///
/// View server leaderboards for different statistics
/// `leaderboard [user (optional, defaults to self)] [refresh]`
#[poise::command(prefix_command, guild_only, slash_command, category = "General")]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "The user to view statistics of, defaults to self"] user: Option<Member>,
    #[description = "Whether to update the counts. NOTE: This could take a while"] refresh: bool,
) -> Result<(), Error> {
    if EmojiCache::is_running() {
        ctx.send_simple(
            true,
            "The leaderboard is already being updated",
            Some("Please try running the command later again"),
            ctx.data().colors.emoji_cache_inaccessible().await,
        )
        .await?;
        return Ok(());
    }
    ctx.defer().await?;
    // Get the emoji data
    let mut data = EmojiCache::new(&ctx.data().pool);
    if refresh {
        data.update_emoji_cache_poise(&ctx, false).await?;
    }
    let mut data = data.get_data().await?;

    let harold_emojis = ctx.data().config.harold_emoji().await?;
    let mut user_list = Vec::new();
    let mut members = ctx.guild_id().unwrap().members_iter(ctx).boxed();
    while let Some(member) = members.next().await {
        user_list.push(member?.user.id.0);
    }

    let emoji_list = ctx
        .guild_id()
        .unwrap()
        .emojis(ctx)
        .await?
        .into_iter()
        .map(|e| e.name)
        .collect::<Vec<String>>();
    data.filter(&user_list, &emoji_list);

    let raw_user_messages = data.user_messages();

    // Get the target user
    let target_user = user.unwrap_or(
        ctx.author_member()
            .await
            .ok_or(Error::from("Failed to get member"))?
            .into_owned(),
    );

    // Get the harold counts
    let mut global_harolds = 0;
    let mut user_harolds = 0;
    let mut harold_leaderboard = HashMap::new();
    for harold in harold_emojis {
        if let Some(harolds) = data.user_emojis_hash_emoji_user().get(&harold.to_string()) {
            user_harolds += *harolds.get(&target_user.user.id.0).unwrap_or(&0);
            global_harolds += *harolds.get(&0).unwrap_or(&0);
            for (user, count) in harolds {
                *harold_leaderboard.entry(*user).or_insert(0) += count;
            }
        }
    }

    // Get the harold percentages
    let mut percentage_leaderboard = raw_user_messages
        .iter()
        .filter_map(|(k, v)| {
            if *v < 500 || *k == 0 {
                None
            } else {
                let harold_count = harold_leaderboard.get(k).unwrap_or(&0);
                Some((*k, *harold_count as f32 / *v as f32))
            }
        })
        .collect::<Vec<(u64, f32)>>();

    // Remove the global value and turn it into a vector
    let mut harold_leaderboard = harold_leaderboard
        .into_iter()
        .filter_map(|(k, v)| match k {
            0 => None,
            _ => Some((k, v)),
        })
        .collect::<Vec<(u64, u64)>>();

    // Get the message counts
    let global_messages = raw_user_messages.get(&0).unwrap_or(&0);
    let user_messages = *raw_user_messages.get(&target_user.user.id.0).unwrap_or(&0);
    let mut message_leaderboard = raw_user_messages
        .iter()
        .filter_map(|(k, v)| match k {
            0 => None,
            _ => Some((*k, *v)),
        })
        .collect::<Vec<(u64, u64)>>();

    // Sort them before building the embeds
    harold_leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
    message_leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
    percentage_leaderboard
        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Create the various embeds that can be cycled through
    let mut harold_embed = CreateEmbed::default();
    let mut message_embed = CreateEmbed::default();
    let mut percentage_embed = CreateEmbed::default();
    let mut user_stats = CreateEmbed::default();
    let mut global_stats = CreateEmbed::default();

    // Populate the embeds
    let color = ctx.data().colors.leaderboard_harold_leaderboard().await;
    harold_embed
        .title("Harold message count")
        .description("Leaderboard of users with the highest amounts of harolds in their messages.")
        .color(color)
        .fields((0..10).filter_map(|i| {
            harold_leaderboard
                .get(i)
                .map(|harold| (i + 1, format!("<@{}> - {}", harold.0, harold.1,), false))
        }));
    let color = ctx
        .data()
        .colors
        .leaderboard_message_count_leaderboard()
        .await;
    message_embed
        .title("Message count")
        .description("Leaderboard of users with the highest amounts of messages.")
        .color(color)
        .fields((0..10).filter_map(|i| {
            message_leaderboard
                .get(i)
                .map(|messages| (i + 1, format!("<@{}> - {}", messages.0, messages.1,), false))
        }));
    let color = ctx
        .data()
        .colors
        .leaderboard_harold_percentage_leaderboard()
        .await;
    percentage_embed
        .title("Harold percentage")
        .description("Leaderboard of users with the highest percentages of harold messages. NOTE: Only users with more than 500 messages in total are accounted for to avoid inaccurate results.")
        .color(color)
        .fields((0..10).filter_map(|i| percentage_leaderboard.get(i).map(|percentages| (i + 1, format!("<@{}> - {}%", percentages.0, (percentages.1 * 100.0) as i32,), false))));

    let color = ctx.data().colors.leaderboard_global().await;
    global_stats
        .title("Global statistics")
        .description("Statistics among all users on the server.")
        .field("Messages", global_messages, false)
        .field("Harold messages", global_harolds, false)
        .field(
            "Harold percentage",
            format!(
                "{}%",
                (global_harolds as f32 / *global_messages as f32 * 100.0) as i32
            ),
            false,
        )
        .color(color);

    let color = ctx.data().colors.leaderboard_user_overview().await;
    user_stats
        .title("User statistics")
        .description(format!(
            "Statistics for the selected user (<@{}>)",
            target_user.user.id.0
        ))
        .field(
            "Harold messages",
            format!(
                "{}{}",
                user_harolds,
                match harold_leaderboard
                    .iter()
                    .position(|(user, _)| *user == target_user.user.id.0)
                {
                    Some(index) => format!(", {}. place on the leaderboard", index + 1),
                    None => "".to_string(),
                }
            ),
            false,
        )
        .field(
            "Messages",
            format!(
                "{}{}",
                user_messages,
                match message_leaderboard
                    .iter()
                    .position(|(user, _)| *user == target_user.user.id.0)
                {
                    Some(index) => format!(", {}. place on the leaderboard", index + 1),
                    None => "".to_string(),
                }
            ),
            false,
        )
        .field(
            "Harold percentage",
            format!(
                "{}%{}",
                (user_harolds as f32 / user_messages as f32 * 100.0) as i32,
                match percentage_leaderboard
                    .iter()
                    .position(|(user, _)| *user == target_user.user.id.0)
                {
                    Some(index) => format!(", {}. place on the leaderboard", index + 1),
                    None => "".to_string(),
                }
            ),
            false,
        )
        .color(color);

    // Create a vector of the embeds for easy access later using an index
    let embed_vec = vec![
        user_stats,
        global_stats,
        harold_embed,
        message_embed,
        percentage_embed,
    ];
    // Create the index and max index to be used for looping through the pages
    let mut index = 0;
    let max_index = embed_vec.len() - 1;

    // Send the message containing the first embed (User stats)
    let mut message = ctx
        .send(|m| {
            m.embed(|e| {
                e.clone_from(&embed_vec[index]);
                e
            })
            // Create the 2 buttons for switching between pages
            .components(|c| {
                c.create_action_row(|a| {
                    a.create_button(|b| b.label("Back").custom_id("ttc-leaderboard-back"))
                        .create_button(|b| b.label("Next").custom_id("ttc-leaderboard-next"))
                })
            })
        })
        .await?
        .message()
        .await?
        .into_owned();

    // Listen for the interactions
    while let Some(interaction) = message
        .await_component_interactions(ctx)
        .timeout(Duration::from_secs(300))
        .author_id(ctx.author().id)
        .build()
        .next()
        .await
    {
        // Change the page depending on the button pressed
        match interaction.data.custom_id.as_str() {
            "ttc-leaderboard-back" => {
                if index > 0 {
                    index -= 1;
                } else {
                    index = max_index;
                }
            }
            "ttc-leaderboard-next" => {
                if index < max_index {
                    index += 1;
                } else {
                    index = 0;
                }
            }
            _ => unreachable!(),
        }
        // Edit the message to contain the correct embed
        interaction
            .create_interaction_response(ctx, |i| {
                i.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|d| d.set_embed(embed_vec[index].clone()))
            })
            .await?;
    }
    // Remove the buttons when we are no longer listening for events
    message.edit(ctx, |e| e.components(|c| c)).await?;

    Ok(())
}

/// Help for all or individual commands
///
/// Command to get help for a specific or all commands
/// ``help [command (optional)]``
#[poise::command(prefix_command, slash_command, category = "General")]
pub async fn help(
    ctx: Context<'_>,
    #[description = "A single command to view help of"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    let color_error = ctx.data().colors.general_error().await;
    let color_help = ctx.data().colors.help().await;
    ctx.defer_ephemeral().await?;
    match command {
        Some(command) => {
            // Remove whitespaces that could come from automatic whitespaces on e.g. mobile devices
            let command = command.trim();
            for help_option in ctx.framework().options().commands.iter() {
                if help_option.name == command {
                    let (desc, color) = match help_option.help_text {
                        Some(s) => (s(), color_help),
                        None => (format!("No help available for {}", &command), color_error),
                    };
                    ctx.send(|m| {
                        m.embed(|e| e.title(command).description(desc).color(color))
                            .ephemeral(true)
                    })
                    .await?;
                    return Ok(());
                }
            }
            // The user called help for something that doesn't exist
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("No help available")
                        .description(format!("Couldn't find \"{}\" in commands. You probably specified a command that isn't existing", command))
                        .color(color_error)
                })
                .ephemeral(true)
            })
            .await?;
        }
        None => {
            let commands = &ctx.framework().options().commands;

            let mut categories: HashMap<&str, Vec<&Command<Data, Error>>> = HashMap::new();

            for command in commands {
                let category = command.category.unwrap_or("General");
                let commands = categories.entry(category).or_insert(Vec::new());
                commands.push(command);
            }
            let mut sorted_categories = categories
                .iter()
                .map(|(k, v)| (*k, v))
                .collect::<Vec<(&str, &Vec<&Command<Data, Error>>)>>();
            sorted_categories.sort_by(|a, b| a.0.cmp(b.0));

            ctx.send(|m| {
                m.embed(|e| {
                    e.title("Help")
                        .fields(sorted_categories.iter().map(|(category, commands)| {
                            let mut commands = (*commands).clone();
                            commands.sort_by(|a, b| a.name.cmp(&b.name));
                            let mut command_string = String::new();
                            for command in commands {
                                if command.hide_in_help {
                                    continue;
                                }
                                command_string.push_str(&format!(
                                    "``{}``: {}\n",
                                    command.name,
                                    command
                                        .description
                                        .clone()
                                        .unwrap_or(String::from("No help available"))
                                ));
                            }
                            if command_string.is_empty() {
                                command_string = "No commands available".to_string();
                            }
                            (category, command_string, false)
                        }))
                        .color(color_help)
                })
                .ephemeral(true)
            })
            .await?;
        }
    }
    Ok(())
}

/// Version command
///
/// Command that shows the current git commit of the bot
/// ``version``
#[poise::command(prefix_command, slash_command, category = "General")]
pub async fn version(ctx: Context<'_>) -> Result<(), Error> {
    let commit = env!("CURRENT_COMMIT");
    ctx.send_simple(
        true,
        format!("This bot runs on commit {}", commit),
        Some(format!(
            "You can find the current source code at https://github.com/TheTerminalCafe/ttc-bot/tree/{}",
            commit
        )),
        ctx.data().colors.version().await,
    )
    .await?;

    Ok(())
}
