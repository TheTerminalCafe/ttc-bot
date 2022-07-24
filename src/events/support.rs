use poise::serenity_prelude::{Color, Context, GuildChannel};

use crate::{commands::support::SupportThread, types::Data, utils::helper_functions::embed_msg, unwrap_or_return};

pub async fn thread_update(ctx: &Context, thread: &GuildChannel, data: &Data) {
    // Make sure the updated part is the archived value

    if thread.thread_metadata.unwrap().archived {
        let pool = &data.pool;

        // Get the current thread info from the database
        let mut db_thread = match sqlx::query_as!(
            SupportThread,
            r#"SELECT * FROM ttc_support_tickets WHERE thread_id = $1"#,
            thread.id.0 as i64
        )
        .fetch_one(pool)
        .await
        {
            Ok(thread) => thread,
            Err(_) => return,
        };

        // Make sure the thread isn't marked as solved
        if !db_thread.incident_solved {
            unwrap_or_return!(thread.edit_thread(&ctx, |t| t.archived(false)).await, "Thread unarchival failed");

            // If the unarchival limit has been reached archive the thread for good
            if db_thread.unarchivals >= 3 {
                match embed_msg(
                    ctx,
                    &thread.id,
                    Some("Closing thread"),
                    Some("3 Unarchivals without solving the issue reached. Closing the thread."),
                    Some(Color::DARK_RED),
                    None,
                )
                .await
                {
                    Ok(_) => (),
                    Err(why) => log::error!("Error sending message: {}", why),
                }

                // Mark the thread as solved on the database
                unwrap_or_return!(sqlx::query!(
                    r#"UPDATE ttc_support_tickets SET incident_solved = 't' WHERE incident_id = $1"#,
                    db_thread.incident_id
                )
                .execute(pool)
                .await, "Error writing to database");

                unwrap_or_return!(thread
                    .edit_thread(&ctx, |t| t.archived(true).locked(true))
                    .await, "Thread archival failed");

                return;
            }

            db_thread.unarchivals += 1;

            // Inform the author of the issue about the unarchival
            match thread.send_message(
                ctx, 
                |m| m.embed(
                    |e| 
                        e.title("Thread unarchived")
                            .description("Thread archival prevented, if the issue is solved mark it as such with the slash command `solve`, or `ttc!solve`.")
                    )
                    .content(format!("<@{}>", db_thread.user_id)
                )
            )
            .await
            {
                Ok(_) => (),
                Err(why) => log::error!("Error sending message: {}", why),
            }

            // Update the unarchivals count
            match sqlx::query!(
                r#"UPDATE ttc_support_tickets SET unarchivals = $1 WHERE incident_id = $2"#,
                db_thread.unarchivals,
                db_thread.incident_id
            )
            .execute(pool)
            .await
            {
                Ok(_) => (),
                Err(why) => log::error!("Error writing to database: {}", why),
            }
        }
    }
}
