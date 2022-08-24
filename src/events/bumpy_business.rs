use std::time::Duration;

use poise::serenity_prelude::{Context, Mentionable, Message, MessageType, Timestamp};

use crate::{types::data::Data, unwrap_or_return};

pub async fn message(ctx: &Context, msg: &Message, data: &Data) {
    match msg.kind {
        MessageType::ChatInputCommand => {
            if msg.interaction.as_ref().unwrap().name == "bump" {
                let color = data.colors.bump_message().await;
                match msg.flags {
                    Some(flags) => {
                        if flags.is_empty() {
                            unwrap_or_return!(msg.channel_id.send_message(
                                ctx, 
                                |m| 
                                    m.content(format!("{}", msg.interaction.as_ref().unwrap().user.mention()))
                                        .embed(|e| 
                                            e.title("Bumpy wumpy")
                                                .description("Thank you for bumping the server, we will make sure to remind you 2 hours from now to do that again.")
                                                .timestamp(Timestamp::now())
                                                .color(color)
                                            )
                                        )
                                        .await, "Error sending message");
                            // 2 hours
                            tokio::time::sleep(Duration::from_secs(7200)).await;
                            unwrap_or_return!(msg.channel_id.send_message(
                                ctx, 
                                |m| 
                                    m.content(format!("{}", msg.interaction.as_ref().unwrap().user.mention()))
                                        .embed(|e| 
                                            e.title("It is bumpy time!")
                                                .description("I am once again asking for you to bump our server.")
                                                .timestamp(Timestamp::now())
                                                .color(color)
                                            )
                                        )
                                        .await, "Error sending message");
                        }
                    }
                    None => (),
                }
            }
        }
        _ => (),
    }
}
