use std::time::Duration;

use crate::{
    command_error, get_config,
    types::{Context, Error},
    utils::helper_functions::*,
};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::{Color, GuildChannel, Message, UserId};

// ----------------------------
// Support thread related types
// ----------------------------

#[derive(Debug)]
pub struct SupportThread {
    pub incident_id: i32,
    pub thread_id: i64,
    pub user_id: i64,
    pub incident_time: DateTime<Utc>,
    pub incident_title: String,
    pub incident_solved: bool,
    pub unarchivals: i16,
}

#[derive(Debug)]
struct ThreadId {
    pub thread_id: i64,
}

impl PartialEq for ThreadId {
    fn eq(&self, other: &Self) -> bool {
        if self.thread_id == other.thread_id {
            true
        } else {
            false
        }
    }
}

// ----------------------
// Support group commands
// ----------------------

#[poise::command(slash_command)]
async fn solve(ctx: Context<'_>) -> Result<(), Error> {
    // Get a reference to the database
    let pool = ctx.data().pool;

    // Get the row with the current channel id from the database
    let thread = match sqlx::query_as!(
        SupportThread,
        r#"SELECT * FROM ttc_support_tickets WHERE thread_id = $1"#,
        ctx.channel_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(thread) => thread,
        Err(why) => {
            return command_error!(format!("Unable to read from database: {}", why));
        }
    };

    // Make sure thread is not yet solved
    if thread.incident_solved {
        embed_msg(
            ctx,
            &ctx.channel_id,
            Some("Thread already solved"),
            Some(&format!(
                "Thread already solved by {} at {}",
                match UserId(thread.user_id as u64).to_user(ctx).await {
                    Ok(user) => user.tag(),
                    Err(_) => "Unknown".to_string(),
                },
                thread.incident_time
            )),
            Some(Color::RED),
            None,
        )
        .await?;
    }

    // Update the state to be solved
    match sqlx::query!(
        r#"UPDATE ttc_support_tickets SET incident_solved = 't' WHERE thread_id = $1"#,
        ctx.channel_id.0 as i64
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(why) => {
            return command_error!(format!("Error reading from database: {}", why));
        }
    }

    embed_msg(
        ctx,
        &ctx.channel_id,
        Some("Great!"),
        Some("Now that the issue is solved, you can give back to society and send the solution after this message."),
        Some(Color::FOOYOO),
        None
        )
        .await?;

    match wait_for_message(ctx, ctx., Duration::from_secs(300)).await {
        Ok(_) => (),
        Err(_) => {
            embed_msg(
                ctx,
                &msg.channel_id,
                Some("Fine."),
                Some("Closing the thread anyway."),
                Some(Color::DARK_RED),
                None,
            )
            .await?;
        }
    }

    let mut new_thread_name = format!(
        "[SOLVED] [{}] {}",
        thread.incident_id, thread.incident_title
    );
    // Make sure the channel name fits in discord channel name character limits
    new_thread_name.truncate(100);

    // Archive the thread after getting the solution
    msg.channel_id
        .edit_thread(ctx, |t| t.name(new_thread_name).archived(true))
        .await?;

    Ok(())
}

#[command]
#[sub_commands(id, title)]
#[usage("<id|title>")]
#[checks(is_in_either)]
async fn search(ctx: &Context, msg: &Message) -> CommandResult {
    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Missing subcommand"),
        Some("Use search with one of the subcommands. (id, title)"),
        Some(Color::RED),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[description("Search for titles containing specified strings from the database. Quotes allow for spaces in naming.")]
#[usage("<list of strings to search for>")]
#[checks(is_in_either)]
#[min_args(1)]
async fn title(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    args.quoted(); // Parse the arguments respecting quoted strings

    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    let mut was_found = false;

    // Loop through the arguments and with each iteration search for them from the database, if
    // found send a message with the information about the ticket
    for arg in args.iter() {
        let arg: String = match arg {
            Ok(arg) => arg,
            Err(why) => {
                embed_msg(
                    ctx,
                    &msg.channel_id,
                    Some("Parsing error"),
                    Some(&format!("Unable to parse argument: {}", why)),
                    Some(Color::RED),
                    None,
                )
                .await?;
                continue;
            }
        };
        let threads = sqlx::query_as!(
            SupportThread,
            r#"SELECT * FROM ttc_support_tickets WHERE incident_title LIKE CONCAT('%', $1::text, '%')"#,
            arg
        )
        .fetch_all(pool)
        .await?;

        for thread in &threads {
            support_ticket_msg(ctx, &msg.channel_id, thread).await?;
            was_found = true;
        }
    }

    // If nothing was found reply with this
    if !was_found {
        embed_msg(
            ctx,
            &msg.channel_id,
            Some("Nothing found"),
            Some("No support ticket found for provided arguments."),
            Some(Color::RED),
            None,
        )
        .await?;
    }

    Ok(())
}

#[command]
#[description("Search for specific id from the database")]
#[usage("<id of support ticket>")]
#[checks(is_in_either)]
#[min_args(1)]
async fn id(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Loop through the arguments and search in each iteration
    for arg in args.iter() {
        let arg: String = match arg {
            Ok(arg) => arg,
            Err(why) => {
                log::error!("Failure getting argument: {}", why);
                continue;
            }
        };
        let id = match arg.parse::<u32>() {
            Ok(id) => id,
            Err(why) => {
                embed_msg(
                    ctx,
                    &msg.channel_id,
                    Some("Parsing error"),
                    Some(&format!("Failure parsing id [{}]: {}", arg, why)),
                    Some(Color::RED),
                    None,
                )
                .await?;
                continue;
            }
        };

        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();

        // Get the support ticket from the database
        let thread = match sqlx::query_as!(
            SupportThread,
            r#"SELECT * FROM ttc_support_tickets WHERE incident_id = $1"#,
            id as i32,
        )
        .fetch_one(pool)
        .await
        {
            Ok(thread) => thread,
            Err(_) => {
                embed_msg(
                    ctx,
                    &msg.channel_id,
                    Some("Nothing found"),
                    Some(&format!("No support ticket found for id [{}].", id)),
                    Some(Color::RED),
                    None,
                )
                .await?;
                continue;
            }
        };

        support_ticket_msg(ctx, &msg.channel_id, &thread).await?;
    }
    Ok(())
}

#[command]
#[description("List tickets based on subcommand")]
#[usage("<active>")]
#[sub_commands(active)]
#[checks(is_in_support_channel)]
async fn list(ctx: &Context, msg: &Message) -> CommandResult {
    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Missing subcommand"),
        Some("Use list with one of the subcommands. (active)"),
        Some(Color::RED),
        None,
    )
    .await?;

    Ok(())
}

#[command]
#[description("List all active tickets")]
#[checks(is_in_support_channel)]
async fn active(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    let threads = sqlx::query_as!(
        SupportThread,
        r#"SELECT * FROM ttc_support_tickets WHERE incident_solved = 'f'"#
    )
    .fetch_all(pool)
    .await?;

    if threads.len() == 0 {
        embed_msg(
            ctx,
            &msg.channel_id,
            Some("Nothing found"),
            Some("No active issues found"),
            Some(Color::BLUE),
            None,
        )
        .await?;
    } else {
        for thread in threads {
            support_ticket_msg(ctx, &msg.channel_id, &thread).await?;
        }
    }

    Ok(())
}

// ----------------------------
// Checks (for channel ids etc)
// ----------------------------

// Check for making sure command originated from the set support channel
#[check]
#[display_in_help(false)]
async fn is_in_support_channel(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let config = get_config!(ctx, {
        return Err(Reason::Log("Database error.".to_string()));
    });

    if config.support_channel as u64 == msg.channel_id.0 {
        return Ok(());
    }

    Err(Reason::Log(format!(
        "{} called outside support channel",
        msg.content
    )))
}

// Check for making sure command originated from one of the known support threads in the database
#[check]
#[display_in_help(false)]
async fn is_in_support_thread(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    // Get the thread ids from the database
    let support_thread_ids =
        sqlx::query_as!(ThreadId, r#"SELECT thread_id FROM ttc_support_tickets"#)
            .fetch_all(pool)
            .await
            .unwrap();

    // Make a ThreadId object out of the channel id for easier comparison
    let channel_id = ThreadId {
        thread_id: msg.channel_id.0 as i64,
    };

    // Check if the id is contained in the support thread ids
    if !support_thread_ids.contains(&channel_id) {
        return Err(Reason::Log(format!(
            "{} called outside a support thread",
            msg.content
        )));
    }

    Ok(())
}

#[check]
#[display_in_help(false)]
async fn is_in_either(
    ctx: &Context,
    msg: &Message,
    args: &mut Args,
    options: &CommandOptions,
) -> Result<(), Reason> {
    if is_in_support_channel(ctx, msg, args, options).await.is_ok()
        || is_in_support_thread(ctx, msg, args, options).await.is_ok()
    {
        return Ok(());
    }

    Err(Reason::Log(format!(
        "{} called outside wither a support thread or the support channel",
        msg.content
    )))
}

#[check]
#[display_in_help(false)]
async fn is_currently_questioned(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let data = ctx.data.read().await;
    let users_currently_questioned = data.get::<UsersCurrentlyQuestionedType>().unwrap();

    if users_currently_questioned.contains(&msg.author.id) {
        return Err(Reason::Log("User tried to open a new support ticket while creation of another one was still ongoing".to_string()));
    }
    Ok(())
}

// ------------------------------------
// Support group related event handling
// ------------------------------------

pub async fn thread_update(ctx: &Context, thread: &GuildChannel) {
    // Make sure the updated part is the archived value
    if thread.thread_metadata.unwrap().archived {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();

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
            match thread.edit_thread(&ctx, |t| t.archived(false)).await {
                Ok(_) => (),
                Err(why) => {
                    log::error!("Thread unarchival failed: {}", why);
                    return;
                }
            }

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
                match sqlx::query!(
                    r#"UPDATE ttc_support_tickets SET incident_solved = 't' WHERE incident_id = $1"#, 
                    db_thread.incident_id
                )
                .execute(pool)
                .await
                {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("Error writing to database: {}", why);
                        return;
                    }
                }

                match thread.edit_thread(&ctx, |t| t.archived(true)).await {
                    Ok(_) => (),
                    Err(why) => {
                        log::error!("Thread archival failed: {}", why);
                        return;
                    }
                }
                return;
            }

            db_thread.unarchivals += 1;

            // Inform the author of the issue about the unarchival
            match thread.send_message(ctx, |m| m.embed(|e| e.title("Thread unarchived").description("Thread archival prevented, if the issue is solved mark it as such with `ttc!support solve`.")).content(format!("<@{}>", db_thread.user_id)))
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
