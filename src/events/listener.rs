use poise::{
    serenity_prelude::Context,
    Event::{self, *},
    FrameworkContext,
};

use crate::{types::data::Data, Error};

pub async fn listener(
    ctx: &Context,
    event: &Event<'_>,
    framework_context: FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Message { new_message } => {
            crate::events::conveyance::message(ctx, new_message, data).await;
            crate::events::bumpy_business::message(ctx, new_message, data).await;
            crate::events::bee::message(ctx, new_message, data).await;
            crate::events::easter_egg::message(ctx, new_message, data, &framework_context).await;
        }
        MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            crate::events::conveyance::message_delete(ctx, channel_id, deleted_message_id, data)
                .await;
            crate::events::emoji_cache::message_delete(
                ctx,
                guild_id,
                channel_id,
                deleted_message_id,
                data,
            )
            .await;
        }
        MessageDeleteBulk {
            channel_id,
            multiple_deleted_messages_ids,
            guild_id: _,
        } => {
            crate::events::conveyance::message_delete_bulk(
                ctx,
                channel_id,
                multiple_deleted_messages_ids,
                data,
            )
            .await;
        }
        MessageUpdate {
            old_if_available: _,
            new,
            event,
        } => {
            // IMPORTANT: conveyance should be called last since it overrides the old message in
            // the DB
            crate::events::emoji_cache::message_update(ctx, new, event, data).await;
            crate::events::conveyance::message_update(ctx, new, event, data).await;
        }
        GuildMemberAddition { new_member } => {
            crate::events::conveyance::guild_member_addition(ctx, new_member, data).await;
        }
        GuildMemberRemoval {
            guild_id: _,
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
            guild_id: _,
            banned_user,
        } => {
            crate::events::conveyance::guild_ban_addition(ctx, banned_user, data).await;
        }
        GuildBanRemoval {
            guild_id: _,
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
        ThreadUpdate { thread } => {
            crate::events::support::thread_update(ctx, thread, data).await;
        }

        _ => (),
    }

    Ok(())
}
