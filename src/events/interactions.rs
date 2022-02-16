use serenity::{
    client::Context,
    model::interactions::{
        Interaction, InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
        InteractionType,
    },
};

pub async fn interaction_create(ctx: &Context, intr: Interaction) {
    match intr.kind() {
        InteractionType::MessageComponent => {
            let intr = intr.message_component().unwrap();
            log::debug!(
                "Interaction created, interaction ID: {}, component ID: {}",
                intr.id,
                intr.data.custom_id
            );

            if intr.user.has_role(ctx, intr.guild_id, "Verified").await {}

            intr.create_interaction_response(ctx, |i| {
                i.interaction_response_data(|r| {
                    r.content("yes")
                        .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                })
            })
            .await
            .unwrap();
        }
        _ => (),
    }
}
