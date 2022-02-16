use crate::typemap::{config::Config, types::PgPoolType};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{
        id::RoleId,
        interactions::{
            Interaction, InteractionApplicationCommandCallbackDataFlags, InteractionType,
        },
    },
    utils::Color,
};

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
                    let config = {
                        let data = ctx.data.read().await;
                        let pool = data.get::<PgPoolType>().unwrap();
                        Config::get_from_db(&pool).await.unwrap()
                    };

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
                                    Ok(_) => (),
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
                None => {
                    log::warn!("Interaction created outside a server");
                }
            }
        }
        _ => (),
    }
}
