use std::time::Duration;

use crate::{get_config, typemap::types::PgPoolType};
use rand::seq::SliceRandom;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        id::{ChannelId, RoleId},
        interactions::{
            message_component::ActionRowComponent, Interaction,
            InteractionApplicationCommandCallbackDataFlags, InteractionType,
        },
    },
    prelude::Mentionable,
    utils::Color,
};

// Macro to quickly check if a user has a certain role
macro_rules! check_user_role {
    ( $ctx:expr, $user:expr, $guild_id:expr, $role_id:expr ) => {
        match $user.has_role($ctx, $guild_id, $role_id).await {
            Ok(result) => result,
            Err(why) => {
                log::error!("Error checking for user {} roles: {}", $user.tag(), why);
                return;
            }
        }
    };
}

pub async fn interaction_create(ctx: &Context, intr: Interaction) {
    match intr.kind() {
        InteractionType::MessageComponent => {
            let intr = intr.message_component().unwrap();
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
                            let config = get_config!(ctx);

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
                                        match intr.create_interaction_response(ctx, |i| {
                                    i.interaction_response_data(|r| {
                                        r.create_embed(|e: &mut CreateEmbed| e.title("Verified!").description("Successfully verified, enjoy your stay!").color(Color::FOOYOO))
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                    })
                                })
                                .await {
                                    Ok(_) => {
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        let welcome_message = config
                                            .welcome_messages
                                            .choose(&mut rand::thread_rng())
                                            .unwrap();
                                        let welcome_message = welcome_message.replace("%user%", &intr.user.mention().to_string());

                                        match ChannelId(config.welcome_channel as u64)
                                            .send_message(ctx, |m| m.content(welcome_message))
                                            .await
                                        {
                                            Ok(_) => (),
                                            Err(why) => {
                                                log::error!("Error sending message: {}", why);
                                                return;
                                            }
                                        }
                                    }
                                    Err(why) => {
                                        log::error!("Unable to respond to interaction: {}", why);
                                    }
                                }
                                    }
                                    Err(why) => {
                                        log::error!("Unable to add verified role: {}", why);
                                        return;
                                    }
                                }
                            } else {
                                // If the user has already verified tell them about it
                                match intr.create_interaction_response(ctx, |i| {
                                    i.interaction_response_data(|r| {
                                        r.create_embed(|e: &mut CreateEmbed| e.title("Verification failed").description("You are already verified! You can't over-verify yourself.").color(Color::RED))
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                                            })}).await {
                                    Ok(_) => (),
                                    Err(why) => {
                                        log::error!("Unable to respond to interaction: {}", why);
                                    }
                                }
                            }
                        }
                        // Self role menu interaction
                        "ttc-bot-self-role-menu" => {
                            // Select the first component from the first action row which *should*
                            // be the selection menu, still check just in case.
                            match &intr.message.components[0].components[0] {
                                ActionRowComponent::SelectMenu(menu) => {
                                    // Get the available self roles from the afformentioned
                                    // component
                                    let available_self_roles: Vec<RoleId> = menu
                                        .options
                                        .iter()
                                        .map(|option| option.value.parse::<RoleId>().unwrap())
                                        .collect();

                                    // Get the member from the interaction
                                    let mut member = intr.member.clone().unwrap();

                                    // If user has the role but has not selected it, remove it.
                                    for role in &available_self_roles {
                                        if check_user_role!(
                                            ctx,
                                            intr.user,
                                            intr.guild_id.unwrap(),
                                            role
                                        ) && !intr.data.values.contains(&role.to_string())
                                        {
                                            match member.remove_role(ctx, role).await {
                                                Ok(_) => (),
                                                Err(why) => {
                                                    log::error!(
                                                        "Error removing role {} from user {}: {}",
                                                        role,
                                                        intr.user.tag(),
                                                        why
                                                    )
                                                }
                                            }
                                        // If user does not have the role but has selected it, add
                                        // it.
                                        } else if !check_user_role!(
                                            ctx,
                                            intr.user,
                                            intr.guild_id.unwrap(),
                                            role
                                        ) && intr.data.values.contains(&role.to_string())
                                        {
                                            match member.add_role(ctx, role).await {
                                                Ok(_) => (),
                                                Err(why) => {
                                                    log::error!(
                                                        "Error adding role {} to user {}: {}",
                                                        role,
                                                        intr.user.tag(),
                                                        why
                                                    );
                                                    return;
                                                }
                                            }
                                        }
                                    }

                                    // Notify the user that their selection of self roles has been
                                    // modified
                                    match intr.create_interaction_response(ctx, |i| i.interaction_response_data(|r| r.create_embed(|e| e.color(Color::FOOYOO).title("Self roles modified").description("Self role modifications successfully completed")).flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL))).await {
                                        Ok(_) => (),
                                        Err(why) => {
                                            log::error!("Failed to respond to interaction user: {}", why);
                                            return;
                                        }
                                    }
                                }
                                _ => {
                                    // In case that for some reason a random component uses this
                                    // id, should never happen but we can never be certain
                                    log::warn!(
                                        "Invalid component type for id \"ttc-bot-self-role-menu\""
                                    );
                                }
                            }
                        }
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
