
use poise::serenity_prelude::{Context, Interaction, InteractionType};
use crate::{types::Data};

// Macro to quickly check if a user has a certain role
macro_rules! check_user_role {
    ( $ctx:expr, $user:expr, $guild_id:expr, $role_id:expr ) => {
        match $user.has_role($ctx, $guild_id, $role_id).await {
            Ok(result) => result,
            Err(why) => {
                return command_error!("Error checking for user {} roles: {}", $user.tag(), why);
            }
        }
    };
}

pub async fn interaction_create(ctx: &Context, intr: &Interaction, data: &Data) {
    match intr.kind() {
        InteractionType::MessageComponent => {
            let intr = match intr.clone().message_component() {
                Some(intr) => intr,
                None => return,
            };
            log::info!(
                "Interaction created, interaction ID: {}, component ID: {}",
                intr.id,
                intr.data.custom_id
            );

            // Make sure the interaction happened inside a guild
            match intr.guild_id {
                Some(_) => {
                    match &intr.data.custom_id[..] {
                        // The interaction for the verification button
                        "ttc-bot-verification-button" => {
                            match interactions::verification_button(ctx, intr, data).await {
                                Ok(_) => (),
                                Err(why) => {
                                    log::error!(
                                        "Error completing verification interaction: {}",
                                        why
                                    );
                                }
                            }
                        }
                        // Self role menu interaction
                        "ttc-bot-self-role-menu" => match interactions::self_role_menu(ctx, intr)
                            .await
                        {
                            Ok(_) => (),
                            Err(why) => {
                                log::error!("Error completing self role menu interaction: {}", why);
                            }
                        },
                        "ttc-bot-ticket-button" => match interactions::ticket_button(ctx, &intr, data)
                            .await
                        {
                            Ok(_) => (),
                            Err(why) => {
                                {
                                    let mut users_currently_questioned = data.users_currently_questioned.lock().await;
                                    users_currently_questioned.retain(|uid| uid != &intr.user.id);
                                }
                                match intr.edit_original_interaction_response(
                                    ctx,
                                    |i| 
                                        i.embed(|e| 
                                            e.title("Something went wrong.")
                                                .description(why)
                                        )
                                    ).await {
                                    Ok(_) => (),
                                    Err(why) => {
                                        log::error!("Error editing interaction response: {}", why);
                                    }
                                }
                            }
                        },
                        _ => {
                            log::warn!("Unknown interaction created");
                        }
                    }
                }
                None => {
                    log::warn!("Interaction created outside a server");
                }
            }
        }
        _ => (),
    }
}

// Module for the separate interaction functions, to keep the main interaction functions clean
mod interactions {
    use chrono::Utc;
    use rand::prelude::SliceRandom;
    use poise::serenity_prelude::{
        CreateEmbed,
        Context,
        ChannelId, 
        RoleId,
        ActionRowComponent, 
        MessageComponentInteraction,
        InteractionApplicationCommandCallbackDataFlags, 
        InteractionResponseType,
        Mentionable,
        Color,
    };
    use std::time::Duration;

    use crate::{
        command_error, get_config,
        groups::support::SupportThread,
        utils::helper_functions::get_message_reply, types::{Error, Data},
    };

    // Interaction for the verification button
    pub async fn verification_button(
        ctx: &Context,
        intr: MessageComponentInteraction,
        data: &Data,
    ) -> Result<(), Error> {
        // Defer the reply to avoid possible issues
        intr.create_interaction_response(ctx, |i| {
            i.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                .interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
        })
        .await?;

        let config = get_config!(data, { return command_error!("Failed to get config") });

        // Check if the user already has the verified role
        if !intr
            .member
            .clone()
            .unwrap()
            .roles
            .contains(&RoleId(config.verified_role as u64))
        {
            match intr
                .member
                .clone()
                .unwrap()
                .add_role(ctx, &RoleId(config.verified_role as u64))
                .await
            {
                Ok(_) => {
                    // Send a message to the user to acknowledge the verification
                    match intr
                        .edit_original_interaction_response(ctx, |i| {
                            i.embed(|e: &mut CreateEmbed| {
                                e.title("Verified!")
                                    .description("Successfully verified, enjoy your stay!")
                                    .color(Color::FOOYOO)
                            })
                        })
                        .await
                    {
                        Ok(_) => {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            let welcome_message = config
                                .welcome_messages
                                .choose(&mut rand::thread_rng())
                                .unwrap();
                            let welcome_message =
                                welcome_message.replace("%user%", &intr.user.mention().to_string());

                            ChannelId(config.welcome_channel as u64)
                                .send_message(ctx, |m| m.content(welcome_message))
                                .await?;
                        }
                        Err(why) => {
                            log::error!("Unable to respond to interaction: {}", why);
                        }
                    }
                }
                Err(why) => {
                    return command_error!("Unable to add verified role: {}", why);
                }
            }
        } else {
            // If the user has already verified tell them about it
            intr.edit_original_interaction_response(ctx, |i| {
                    i.embed(|e: &mut CreateEmbed| {
                        e.title("Verification failed")
                            .description(
                                "You are already verified! You can't over-verify yourself.",
                            )
                            .color(Color::RED)
                    })
                })
            .await?;
        }
        Ok(())
    }
    
    // Interaction for the self role menu
    pub async fn self_role_menu(ctx: &Context, intr: MessageComponentInteraction) -> Result<(), Error>  {
        // Select the first component from the first action row which *should*
        // be the selection menu, still check just in case.
        match &intr.message.components[0].components[0] {
            ActionRowComponent::SelectMenu(menu) => {
                intr.create_interaction_response(ctx, |i| {
                    i.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        .interaction_response_data(|d| {
                            d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        })
                })
                .await?;

                // Get the available self roles from the afformentioned
                // component
                let available_self_roles: Vec<RoleId> = menu
                    .options
                    .iter()
                    .map(|option| option.value.parse::<RoleId>().unwrap())
                    .collect();

                // Get the member from the interaction
                let mut member = intr.member.clone().unwrap();

                let mut roles_to_remove: Vec<RoleId> = Vec::new();
                let mut roles_to_add: Vec<RoleId> = Vec::new();

                // If user has the role but has not selected it, remove it.
                for role in &available_self_roles {
                    if check_user_role!(ctx, intr.user, intr.guild_id.unwrap(), role)
                        && !intr.data.values.contains(&role.to_string())
                    {
                        roles_to_remove.push(*role);
                    // If user does not have the role but has selected it, add
                    // it.
                    } else if !check_user_role!(ctx, intr.user, intr.guild_id.unwrap(), role)
                        && intr.data.values.contains(&role.to_string())
                    {
                        roles_to_add.push(*role);
                    }
                }
                if roles_to_add.len() > 0 {
                    member.add_roles(ctx, &roles_to_add).await?;
                }
                if roles_to_remove.len() > 0 {
                    member.remove_roles(ctx, &roles_to_remove).await?;
                }

                // Notify the user that their selection of self roles has been
                intr.edit_original_interaction_response(ctx, |i| {
                    i.embed(|e| {
                        e.color(Color::FOOYOO)
                            .title("Self roles modified")
                            .description("Self role modifications successfully completed")
                    })
                })
                .await?;
            }
            _ => {
                // In case that for some reason a random component uses this
                // id, should never happen but we can never be certain
                log::warn!("Invalid component type for id \"ttc-bot-self-role-menu\"");
            }
        }
        Ok(())
    }

    pub async fn ticket_button(ctx: &Context, intr: &MessageComponentInteraction, data: &Data) -> Result<(), Error> {
        // Defer the reply initially to avoid getting the interaction invalidated
        intr.create_interaction_response(ctx, |i| {
            i.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                .interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
        })
        .await?;

        {
            let mut users_currently_questioned = data.users_currently_questioned.lock().await;
            if users_currently_questioned.contains(&intr.user.id) {
                match intr.edit_original_interaction_response(ctx, |i| {
                    i.embed(|e| {
                        e.color(Color::FOOYOO)
                            .title("You are already opening a ticket!")
                            .description("Please finish that before opening a new one.")
                    })
                }).await {
                    Ok(_) => (),
                    Err(_) => (),
                }
                return Ok(())
            } else {
                users_currently_questioned.push(intr.user.id);
            }
        }

        let config = get_config!(data, { return command_error!("Failed to get config") });

        let support_channel = ChannelId(config.support_channel as u64);

        let mut thread_msg = support_channel
            .send_message(ctx, |m| m.embed(|e| e.title("Pending info...")))
            .await?;

        // Here the data variable doesn't live long and a read lock is much better for smooth
        // operation, so it can be locked "globally" like this
        let thread = support_channel
            .create_public_thread(ctx, thread_msg.id, |ct| ct.name("Pending title..."))
            .await?;

        intr.edit_original_interaction_response(ctx, |i| {
            i.embed(|e| {
                e.title("Ticket created")
                    .description(format!("A ticket has been created for you in <#{}>", thread.id))
                    .color(Color::FOOYOO)
            })
        }).await?;

        let thread_name = match get_message_reply(
            ctx, 
            &thread.id,
            &intr.user, 
            |m| 
                m.content(format!("{}", intr.user.mention()))
                    .embed(|e| 
                        e.title("Provide a title for the issue").description("By sending it as a message in this thread, max length ~100 characters, time limit 5 minutes")), 
            Duration::from_secs(300), data).await {
            Ok(response) => response,
            Err(why) => {
                thread.delete(ctx).await?;
                thread_msg.delete(ctx).await?;
                return command_error!("Failed to get message reply: {}", why);
            }
        };

        let description = match get_message_reply(
            ctx, 
            &thread.id,
            &intr.user,
            |m| 
                m.embed(|e| 
                    e.title("Provide a description for the issue")
                        .description(
                            "By sending it as a message in this thread, max length 4096 characters, time limit 10 minutes\n\n
                            Include information like:\n\n
                            - Short description of the issue\n
                            - System information (OS, CPU, GPU, etc.)\n
                            - Anything else that may be relevant for the issue.")), 
            Duration::from_secs(600), data
        ).await {
            Ok(description) => description,
            Err(why) => {
                thread.delete(ctx).await?;
                thread_msg.delete(ctx).await?;
                return command_error!("Failed to get message reply: {}", why);
            }
        };

        let mut thread_name_safe = data.thread_name_regex
            .replace_all(&thread_name, "")
            .to_string();
        thread_name_safe.truncate(100);

        {
            let mut users_currently_questioned = data.users_currently_questioned.lock().await;
            users_currently_questioned.retain(|uid| uid != &intr.user.id);
        }

        // Insert the gathered information into the database and return the newly created database
        // entry for it's primary key to be added to the support thread title

        let pool = &data.pool;

        let db_thread = match sqlx::query_as!(
            SupportThread,
            r#"INSERT INTO ttc_support_tickets (thread_id, user_id, incident_time, incident_title, incident_solved, unarchivals) VALUES($1, $2, $3, $4, $5, $6) RETURNING *"#,
            thread.id.0 as i64,
            intr.user.id.0 as i64,
            Utc::now(),
            thread_name_safe,
            false,
            0,
        )
        .fetch_one(pool)
        .await {
            Ok(thread) => thread,
            Err(why) => {
                return command_error!(format!("Error writing into database: {}", why));
            }
        };

        let mut new_thread_name = format!("[{}] {}", db_thread.incident_id, thread_name_safe);
        new_thread_name.truncate(100);

        thread
            .id
            .edit_thread(ctx, |t| t.name(&new_thread_name))
            .await?;

        let user_name = match &intr.member {
            Some(member) => member.nick.clone().unwrap_or(intr.user.name.clone()),
            None => intr.user.name.clone(),
        };
        
        thread_msg.edit(ctx, |m| m.content("").embed(|e|
            e.title(new_thread_name)
                .description(description)
                .author(|a| a.name(user_name).icon_url(intr.user.face()))
                .color(Color::FOOYOO)
        )).await?;


        Ok(())
    }
}
