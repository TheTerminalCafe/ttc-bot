use crate::types::data::Data;
use poise::serenity_prelude::{Context, Interaction, InteractionType};

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
            log::debug!(
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
                        "ttc-bot-self-role-menu" => {
                            match interactions::self_role_menu(ctx, intr, data).await {
                                Ok(_) => (),
                                Err(why) => {
                                    log::error!(
                                        "Error completing self role menu interaction: {}",
                                        why
                                    );
                                }
                            }
                        }
                        "ttc-bot-ticket-button" => {
                            match interactions::ticket_button(ctx, &intr).await {
                                Ok(_) => (),
                                Err(why) => {
                                    log::error!(
                                        "Error completing ticket button interaction: {}",
                                        why
                                    );
                                    match intr
                                        .edit_original_interaction_response(ctx, |i| {
                                            i.embed(|e| {
                                                e.title("Something went wrong.").description(why)
                                            })
                                        })
                                        .await
                                    {
                                        Ok(_) => (),
                                        Err(why) => {
                                            log::error!(
                                                "Error editing interaction response: {}",
                                                why
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
                None => {
                    log::warn!("Interaction created outside a server");
                }
            }
        }
        InteractionType::ModalSubmit => {
            let intr = match intr.clone().modal_submit() {
                Some(intr) => intr,
                None => return,
            };

            match &intr.data.custom_id[..] {
                "ttc-bot-ticket-modal" => {
                    match interactions::ticket_modal(ctx, &intr, data).await {
                        Ok(_) => (),
                        Err(why) => {
                            let color = data.colors.input_error().await;
                            match intr
                                .edit_original_interaction_response(ctx, |m| {
                                    m.embed(|e| {
                                        e.title("An error occurred")
                                            .description(format!("{}", why))
                                            .color(color)
                                    })
                                })
                                .await
                            {
                                Ok(_) => (),
                                Err(why) => log::error!("Failed to send error message: {}", why),
                            }
                            log::warn!("Failed to complete support ticket creation: {}", why);
                            return;
                        }
                    }
                }
                _ => (),
            }
        }
        _ => (),
    }
}

// Module for the separate interaction functions, to keep the main interaction functions clean
mod interactions {
    use chrono::Utc;
    use poise::serenity_prelude::{
        ActionRowComponent, ChannelId, Context, CreateEmbed, InputTextStyle,
        InteractionApplicationCommandCallbackDataFlags, InteractionResponseType, Mentionable,
        MessageComponentInteraction, ModalSubmitInteraction, RoleId,
    };
    use rand::prelude::SliceRandom;
    use std::time::Duration;

    use crate::{command_error, commands::support::SupportThread, types::data::Data, Error};

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

        // Make sure accounts that enter are older than 7 days
        if Utc::now().timestamp()
            - intr
                .member
                .clone()
                .unwrap()
                .user
                .created_at()
                .unix_timestamp()
            < chrono::Duration::days(7).num_seconds()
        {
            let color = data.colors.general_error().await;
            intr.edit_original_interaction_response(ctx, |i| {
                i.embed(|e| {
                    e.title("An error occurred")
                        .description("Something went wrong.")
                        .color(color)
                })
            })
            .await?;
            return Ok(());
        }

        let color = data.colors.verify_color().await;
        // Check if the user already has the verified role
        if !intr
            .member
            .clone()
            .unwrap()
            .roles
            .contains(&RoleId(data.config.verified_role().await? as u64))
        {
            match intr
                .member
                .clone()
                .unwrap()
                .add_role(ctx, &RoleId(data.config.verified_role().await? as u64))
                .await
            {
                Ok(_) => {
                    // Send a message to the user to acknowledge the verification
                    match intr
                        .edit_original_interaction_response(ctx, |i| {
                            i.embed(|e: &mut CreateEmbed| {
                                e.title("Verified!")
                                    .description("Successfully verified, enjoy your stay!")
                                    .color(color)
                            })
                        })
                        .await
                    {
                        Ok(_) => {
                            tokio::time::sleep(Duration::from_secs(2)).await;

                            let welcome_message = data.config.welcome_message().await?;
                            let welcome_message =
                                welcome_message.choose(&mut rand::thread_rng()).unwrap();
                            let welcome_message =
                                welcome_message.replace("%user%", &intr.user.mention().to_string());

                            ChannelId(data.config.welcome_channel().await? as u64)
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
            let color = data.colors.general_error().await;
            // If the user has already verified tell them about it
            intr.edit_original_interaction_response(ctx, |i| {
                i.embed(|e: &mut CreateEmbed| {
                    e.title("Verification failed")
                        .description("You are already verified! You can't over-verify yourself.")
                        .color(color)
                })
            })
            .await?;
        }
        Ok(())
    }

    // Interaction for the self role menu
    pub async fn self_role_menu(
        ctx: &Context,
        intr: MessageComponentInteraction,
        data: &Data,
    ) -> Result<(), Error> {
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

                let color = data.colors.selfrole_post_edit_msg().await;
                // Notify the user that their selection of self roles has been
                intr.edit_original_interaction_response(ctx, |i| {
                    i.embed(|e| {
                        e.color(color)
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

    pub async fn ticket_button(
        ctx: &Context,
        intr: &MessageComponentInteraction,
    ) -> Result<(), Error> {
        intr.create_interaction_response(ctx, |i| {
            i.kind(InteractionResponseType::Modal)
                .interaction_response_data(|d| {
                    d.custom_id("ttc-bot-ticket-modal")
                        .title("Support ticket")
                        .components(|c| {
                            c.create_action_row(|a| {
                                a.create_input_text(|t| {
                                    t.label("Title for the support ticket")
                                        .max_length(100)
                                        .custom_id("ttc-bot-ticket-modal-title")
                                        .placeholder("Computer does not turn on")
                                        .required(true)
                                        .style(InputTextStyle::Short)
                                })
                            }).create_action_row(|a| {
                                a.create_input_text(|t| {
                                    t.label("A longer description of the issue")
                                        .max_length(1024)
                                        .required(true)
                                        .custom_id("ttc-bot-ticket-modal-description")
                                        .placeholder("My computer suddenly does not boot up anymore, could be due to it being submerged in water.")
                                        .style(InputTextStyle::Paragraph)
                                })
                            }).create_action_row(|a| {
                                a.create_input_text(|t| {
                                    t.label("System information")
                                        .max_length(1024)
                                        .custom_id("ttc-bot-ticket-modal-systeminfo")
                                        .required(true)
                                        .placeholder("OS: Cursed abomination\nCPU: Raisin 6 9000XT\nGPU: Radon G86\nDE: Elf 42")
                                        .style(InputTextStyle::Paragraph)
                                })
                            })
                        })
                    })
                }).await?;

        Ok(())
    }

    pub async fn ticket_modal(
        ctx: &Context,
        intr: &ModalSubmitInteraction,
        data: &Data,
    ) -> Result<(), Error> {
        // Defer the reply initially to avoid getting the interaction invalidated
        intr.create_interaction_response(ctx, |i| {
            i.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                .interaction_response_data(|d| {
                    d.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
        })
        .await?;

        let mut title = String::new();
        let mut description = String::new();
        let mut system_info = String::new();

        // Extract the data from the components of the modal
        for row in intr.data.components.iter() {
            match &row.components[0] {
                ActionRowComponent::InputText(input) => match input.custom_id.as_str() {
                    "ttc-bot-ticket-modal-title" => {
                        title = data
                            .thread_name_regex
                            .replace_all(&input.value, "")
                            .to_string();
                        if title.trim().is_empty() {
                            return Err(Error::from("The title can't be empty.".to_string()));
                        }
                    }
                    "ttc-bot-ticket-modal-description" => {
                        description = input.value.clone();
                        if description.trim().is_empty() {
                            return Err(Error::from("The description can't be empty.".to_string()));
                        }
                    }
                    "ttc-bot-ticket-modal-systeminfo" => {
                        system_info = input.value.clone();
                        if system_info.trim().is_empty() {
                            return Err(Error::from("The system info can't be empty.".to_string()));
                        }
                    }
                    _ => log::warn!(
                        "Invalid custom id for support ticket modal component: {}",
                        input.custom_id
                    ),
                },
                _ => log::warn!("Invalid component on support ticket modal."),
            }
        }

        let support_channel = ChannelId(data.config.support_channel().await? as u64);
        let color = data.colors.ticket_summary().await;

        let user_name = match &intr.member {
            Some(member) => member.nick.clone().unwrap_or(intr.user.name.clone()),
            None => intr.user.name.clone(),
        };

        let thread_msg = support_channel
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title(&title)
                        .field("Description", description, false)
                        .field("System info", system_info, false)
                        .author(|a| a.name(user_name).icon_url(intr.user.face()))
                        .color(color)
                })
            })
            .await?;

        let thread = support_channel
            .create_public_thread(ctx, thread_msg.id, |ct| ct.name(&title))
            .await?;

        // Insert the gathered information into the database and return the newly created database
        // entry for it's primary key to be added to the support thread title

        let pool = &*data.pool;

        let db_thread = match sqlx::query_as!(
            SupportThread,
            r#"INSERT INTO ttc_support_tickets (thread_id, user_id, incident_time, incident_title, incident_solved, unarchivals) VALUES($1, $2, $3, $4, $5, $6) RETURNING *"#,
            thread.id.0 as i64,
            intr.user.id.0 as i64,
            Utc::now(),
            title,
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

        let mut new_title = format!("[{}] {}", db_thread.incident_id, title);
        new_title.truncate(100);

        thread.id.edit_thread(ctx, |t| t.name(&new_title)).await?;

        intr.edit_original_interaction_response(ctx, |i| {
            i.embed(|e| {
                e.title("Support ticket created")
                    .description(format!("Support ticket created in <#{}>", thread.id.0))
            })
        })
        .await?;

        thread
            .id
            .send_message(ctx, |m| m.content(format!("<@{}>", intr.user.id.0)))
            .await?;

        Ok(())
    }
}
