use crate::{
    command_error,
    traits::{context_ext::ContextExt, readable::Readable},
    Context, Error,
};
use chrono::{DateTime, Utc};
use poise::{serenity_prelude::CreateEmbed, CreateReply};

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

/// Close the current support thread
///
/// Marks the current support threads as solved, and archives it.
/// **NOTE**: This command is only available in support threads.
/// ``solve``
#[poise::command(
    slash_command,
    prefix_command,
    check = "is_in_support_thread",
    category = "Support",
    guild_only
)]
pub async fn solve(ctx: Context<'_>) -> Result<(), Error> {
    // Get a reference to the database
    let pool = &*ctx.data().pool;

    // Get the row with the current channel id from the database
    let thread = match sqlx::query_as!(
        SupportThread,
        r#"SELECT * FROM ttc_support_tickets WHERE thread_id = $1"#,
        ctx.channel_id().0 as i64
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
        ctx.send_simple(
            true,
            "Error",
            Some("This ticket is already solved."),
            ctx.data().colors.input_error().await,
        )
        .await?;
        return Ok(());
    }

    // Update the state to be solved
    match sqlx::query!(
        r#"UPDATE ttc_support_tickets SET incident_solved = 't' WHERE thread_id = $1"#,
        ctx.channel_id().0 as i64
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(why) => {
            return command_error!(format!("Error reading from database: {}", why));
        }
    }

    ctx.send_simple(
        false,
        "Great!",
        Some("Now that this ticket has been solved, this thread will be archived."),
        ctx.data().colors.support_info().await,
    )
    .await?;

    let mut new_thread_name = format!(
        "[SOLVED] [{}] {}",
        thread.incident_id, thread.incident_title
    );
    // Make sure the channel name fits in discord channel name character limits
    new_thread_name.truncate(100);

    // Archive the thread after getting the solution
    ctx.channel_id()
        .edit_thread(ctx, |t| t.name(new_thread_name).archived(true).locked(true))
        .await?;

    Ok(())
}

/// Search for a support ticket
///
/// Search for a support ticket based on either title, id or both.
/// **NOTE**: Either id or title must be provided.
/// ``search [id (optional)] [title (optional)]``
#[poise::command(slash_command, prefix_command, category = "Support")]
pub async fn search(
    ctx: Context<'_>,
    #[description = "Id to search for"]
    #[min = 0]
    id: Option<u32>,
    #[description = "Title to search for"] title: Option<String>,
) -> Result<(), Error> {
    // Let's keep count so we know if one of them was around
    let mut options_used = 0;

    match id {
        Some(id) => {
            search_id(ctx, id).await?;
            options_used += 1;
        }
        None => (),
    }
    match title {
        Some(title) => {
            search_title(ctx, title).await?;
            options_used += 1;
        }
        None => (),
    }
    if options_used == 0 {
        return Err(Error::from(
            "Please provide either an id or a title to search for.",
        ));
    }

    Ok(())
}

async fn search_title(ctx: Context<'_>, title: String) -> Result<(), Error> {
    let pool = &*ctx.data().pool;

    // Loop through the arguments and with each iteration search for them from the database, if
    // found send a message with the information about the ticket
    let threads = sqlx::query_as!(
        SupportThread,
        r#"SELECT * FROM ttc_support_tickets WHERE incident_title LIKE CONCAT('%', $1::text, '%')"#,
        title
    )
    .fetch_all(pool)
    .await?;

    let mut msg = if threads.len() != 0 {
        let color = ctx.data().colors.support_info().await;
        let mut msg = CreateReply::default();
        msg.embed(|e| e.title("List of support tickets found:").color(color));

        for thread in &threads {
            msg.embed(|e| support_ticket_embed(thread, e));
        }
        msg
    } else {
        let color = ctx.data().colors.general_error().await;
        let mut msg = CreateReply::default();
        msg.embed(|e| e.title("No support tickets found.").color(color));
        msg
    };

    ctx.send(|_| &mut msg).await?;

    Ok(())
}

async fn search_id(ctx: Context<'_>, id: u32) -> Result<(), Error> {
    let pool = &*ctx.data().pool;

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
            ctx.send_simple(
                true,
                "No such ticket found",
                None,
                ctx.data().colors.input_error().await,
            )
            .await?;
            return Ok(());
        }
    };

    ctx.send(|m| m.embed(|e| support_ticket_embed(&thread, e)))
        .await?;

    Ok(())
}

// Check for making sure command originated from one of the known support threads in the database
async fn is_in_support_thread(ctx: Context<'_>) -> Result<bool, Error> {
    let pool = &*ctx.data().pool;

    // Get the thread ids from the database
    let support_thread_ids =
        sqlx::query_as!(ThreadId, r#"SELECT thread_id FROM ttc_support_tickets"#)
            .fetch_all(pool)
            .await?;

    // Make a ThreadId object out of the channel id for easier comparison
    let channel_id = ThreadId {
        thread_id: ctx.channel_id().0 as i64,
    };

    // Check if the id is contained in the support thread ids
    Ok(support_thread_ids.contains(&channel_id))
}

// ------------------------------------
// Support group related event handling
// ------------------------------------

fn support_ticket_embed<'a>(
    thread: &SupportThread,
    embed: &'a mut CreateEmbed,
) -> &'a mut CreateEmbed {
    embed
        .title(format!("Support ticket [{}]", thread.incident_id))
        .field("Title:", thread.incident_title.clone(), false)
        .field(
            "Status:",
            format!("Solved: {}", thread.incident_solved,),
            false,
        )
        .field("Timestamp:", thread.incident_time.readable(), false)
        .field("Thread:", format!("<#{}>", thread.thread_id), false)
}
