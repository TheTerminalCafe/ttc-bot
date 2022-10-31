use poise::{
    async_trait,
    serenity_prelude::{Color, CreateEmbed},
    ReplyHandle,
};

use crate::Context;

/// A trait for some shorter message sending functions
#[async_trait]
pub trait ContextExt {
    /// Just send an embed, discarding the `CreateReply` part otherwise. To make the message ephemeral
    /// a boolean argument exists for that.

    async fn send_embed<F: Send>(
        &self,
        ephemeral: bool,
        f: F,
    ) -> Result<ReplyHandle, poise::serenity_prelude::Error>
    where
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed;

    /// Send a simple embed with just a title, description and color. Also contains the ephemeral toggle
    async fn send_simple<T: ToString + Send>(
        &self,
        ephemeral: bool,
        title: T,
        description: Option<T>,
        color: Color,
    ) -> Result<ReplyHandle, poise::serenity_prelude::Error>;
}

#[async_trait]
impl<'a> ContextExt for Context<'a> {
    async fn send_embed<F: Send>(
        &self,
        ephemeral: bool,
        f: F,
    ) -> Result<ReplyHandle, poise::serenity_prelude::Error>
    where
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
    {
        self.send(|m| m.embed(f).ephemeral(ephemeral)).await
    }

    async fn send_simple<T: ToString + Send>(
        &self,
        ephemeral: bool,
        title: T,
        description: Option<T>,
        color: Color,
    ) -> Result<ReplyHandle, poise::serenity_prelude::Error> {
        self.send(|m| {
            m.embed(|e| {
                e.title(title).color(color);
                match description {
                    Some(description) => {
                        e.description(description);
                    }
                    None => (),
                };
                e
            })
            .ephemeral(ephemeral)
        })
        .await
    }
}
