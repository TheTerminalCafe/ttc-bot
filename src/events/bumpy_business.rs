use std::time::Duration;

use poise::serenity_prelude::{Color, Context, Mentionable, Message, MessageType, Timestamp};

use crate::ttc_unwrap;

pub async fn message(ctx: &Context, msg: &Message) {
    match msg.kind {
        MessageType::ChatInputCommand => {
            if msg.interaction.as_ref().unwrap().name == "bump" {
                match msg.flags {
                    Some(flags) => {
                        if flags.is_empty() {
                            ttc_unwrap!(msg.channel_id.send_message(
                                ctx, 
                                |m| 
                                    m.content(format!("{}", msg.interaction.as_ref().unwrap().user.mention()))
                                        .embed(|e| 
                                            e.title("Bumpy wumpy")
                                                .description("Thank you for bumping the server, we will make sure to remind you 2 hours from now to do that again.")
                                                .timestamp(Timestamp::now())
                                                .color(Color::PURPLE)
                                            )
                                        )
                                        .await, "Error sending message");
                            // 2 hours
                            tokio::time::sleep(Duration::from_secs(7200)).await;
                            ttc_unwrap!(msg.channel_id.send_message(
                                ctx, 
                                |m| 
                                    m.content(format!("{}", msg.interaction.as_ref().unwrap().user.mention()))
                                        .embed(|e| 
                                            e.title("It is bumpy time!")
                                                .description("I am once again asking for you to bump our server.")
                                                .timestamp(Timestamp::now())
                                                .color(Color::PURPLE)
                                            )
                                        )
                                        .await, "Error sending message");
                        }
                    }
                    None => ()
                }
            }
        }
        _ => (),
    }
}
