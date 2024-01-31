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
                            match interaction_fns::verification_button(ctx, intr, data).await {
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
                            match interaction_fns::self_role_menu(ctx, intr, data).await {
                                Ok(_) => (),
                                Err(why) => {
                                    log::error!(
                                        "Error completing self role menu interaction: {}",
                                        why
                                    );
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
        _ => (),
    }
}

// Module for the separate interaction functions, to keep the main interaction functions clean
mod interaction_fns {
    use chrono::Utc;
    use poise::serenity_prelude::{
        ActionRowComponent, Context, CreateEmbed, InteractionResponseFlags,
        InteractionResponseType, MessageComponentInteraction, RoleId,
    };

    use crate::{command_error, types::data::Data, Error};

    // Interaction for the verification button
    pub async fn verification_button(
        ctx: &Context,
        intr: MessageComponentInteraction,
        data: &Data,
    ) -> Result<(), Error> {
        // Defer the reply to avoid possible issues
        intr.create_interaction_response(ctx, |i| {
            i.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                .interaction_response_data(|d| d.flags(InteractionResponseFlags::EPHEMERAL))
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
                    if let Err(why) = intr
                        .edit_original_interaction_response(ctx, |i| {
                            i.embed(|e: &mut CreateEmbed| {
                                e.title("Verified!")
                                    .description("Successfully verified, enjoy your stay!")
                                    .color(color)
                            })
                        })
                        .await
                    {
                        log::error!("Unable to respond to interaction: {}", why);
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
                        .interaction_response_data(|d| d.flags(InteractionResponseFlags::EPHEMERAL))
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
                if !roles_to_add.is_empty() {
                    member.add_roles(ctx, &roles_to_add).await?;
                }
                if !roles_to_remove.is_empty() {
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
}
