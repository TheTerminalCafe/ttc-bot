use poise::{serenity_prelude::Context, BoxFuture, Event, Framework};

use crate::types::{Data, Error};

pub async fn listener(
    ctx: &Context,
    event: &Event<'_>,
    framework: &Framework<Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Event::Message { new_message: msg } => {
            crate::events::conveyance::message(ctx, &data.pool, msg).await;
        }
        _ => (),
    }

    Ok(())
}
