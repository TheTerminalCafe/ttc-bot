use poise::{
    serenity_prelude::Context,
    BoxFuture,
    Event::{self, *},
    Framework,
};

use crate::types::{Data, Error};

pub async fn listener(
    ctx: &Context,
    event: &Event<'_>,
    framework: &Framework<Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Message { new_message } => {
            crate::events::conveyance::message(ctx, new_message, data).await;
        }
        MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id: _,
        } => {
            crate::events::conveyance::message_delete(ctx, channel_id, deleted_message_id, data)
                .await;
        }
        MessageUpdate {
            old_if_available,
            new,
            event,
        } => {
            crate::events::conveyance::message_update(ctx, old_if_available, new, event, data)
                .await;
        }
        GuildMemberAddition { new_member } => {
            crate::events::conveyance::guild_member_addition(ctx, new_member, data).await;
        }
        GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available,
        } => {
            crate::events::conveyance::guild_member_removal(
                ctx,
                user,
                member_data_if_available,
                data,
            )
            .await;
        }
        GuildBanAddition {
            guild_id,
            banned_user,
        } => {
            crate::events::conveyance::guild_ban_addition(ctx, banned_user, data).await;
        }
        GuildBanRemoval {
            guild_id,
            unbanned_user,
        } => {
            crate::events::conveyance::guild_ban_removal(ctx, unbanned_user, data).await;
        }
        GuildMemberUpdate {
            old_if_available,
            new,
        } => {
            crate::events::conveyance::guild_member_update(ctx, old_if_available, new, data).await;
        }
        InteractionCreate { interaction } => {
            crate::events::interactions::interaction_create(ctx, interaction, data).await;
        }

        _ => (),
    }

    Ok(())
}