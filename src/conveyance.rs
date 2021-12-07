use serenity::{
    client::Context,
    model::id::{ChannelId, MessageId},
};

// --------------------------------
// Functions for conveyance logging
// --------------------------------

pub fn message_delete(ctx: Content, channel_id: ChannelId, deleted_message_id: MessageId) {}
