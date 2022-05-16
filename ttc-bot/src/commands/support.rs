use crate::{
    command_error,
    types::{Context, Error},
};
use chrono::{DateTime, Utc};
use poise::{
    serenity_prelude::{Color, CreateEmbed},
    CreateReply,
};

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

// TODO: Add help
#[poise::command(
    slash_command,
    prefix_command,
    check = "is_in_support_thread",
    category = "Support",
    guild_only
)]
pub async fn solve(ctx: Context<'_>) -> Result<(), Error> {
    // Get a reference to the database
    let pool = &ctx.data().pool;

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
        ctx.send(|m| {
            m.embed(|e| {
                e.title("Error")
                    .description("This ticket is already solved.")
                    .color(Color::RED)
            })
        })
        .await?;
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

    ctx.send(|m| {
        m.embed(|e| {
            e.title("Great!")
                .description("Now that this ticket has been solved, this thread will be archived.")
                .color(Color::FOOYOO)
        })
    })
    .await?;

    let mut new_thread_name = format!(
        "[SOLVED] [{}] {}",
        thread.incident_id, thread.incident_title
    );
    // Make sure the channel name fits in discord channel name character limits
    new_thread_name.truncate(100);

    // Archive the thread after getting the solution
    ctx.channel_id()
        .edit_thread(ctx.discord(), |t| t.name(new_thread_name).archived(true))
        .await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Support")]
pub async fn search(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Search")
                .description("Search for a support ticket by its ID or title.")
                .color(Color::FOOYOO)
                .field("Usage", "`search <id|title> <argument>`", false)
        })
        .ephemeral(true)
    })
    .await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Support")]
pub async fn title(
    ctx: Context<'_>,
    #[description = "Search for an issue based on a title"] title: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let pool = &ctx.data().pool;

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
        let mut msg = CreateReply::default();
        msg.embed(|e| {
            e.title("List of support tickets found:")
                .color(Color::FOOYOO)
        });

        for thread in &threads {
            msg.embed(|e| support_ticket_embed(thread, e));
        }
        msg
    } else {
        let mut msg = CreateReply::default();
        msg.embed(|e| e.title("No support tickets found.").color(Color::RED));
        msg
    };

    ctx.send(|_| &mut msg).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, category = "Support")]
pub async fn id(
    ctx: Context<'_>,
    #[description = "The numerical id for the thread"]
    #[min = 0]
    id: u32,
) -> Result<(), Error> {
    ctx.defer().await?;

    let pool = &ctx.data().pool;

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
            ctx.send(|m| {
                m.ephemeral(true)
                    .embed(|e| e.title("No such ticket found").color(Color::RED))
            })
            .await?;
            return Ok(());
        }
    };

    ctx.send(|m| m.embed(|e| support_ticket_embed(&thread, e)))
        .await?;

    Ok(())
}

/*#[command]
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
}*/

/*#[command]
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
}*/

// ----------------------------
// Checks (for channel ids etc)
// ----------------------------

// Check for making sure command originated from the set support channel
/*#[check]
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
}*/

// Check for making sure command originated from one of the known support threads in the database
async fn is_in_support_thread(ctx: Context<'_>) -> Result<bool, Error> {
    let pool = &ctx.data().pool;

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

/*#[check]
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
}*/

/*#[check]
#[display_in_help(false)]
async fn is_currently_questioned(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let data = ctx.data.read().await;
    let users_currently_questioned = data.get::<UsersCurrentlyQuestionedType>().unwrap();

    if users_currently_questioned.contains(&msg.author.id) {
        return Err(Reason::Log("User tried to open a new support ticket while creation of another one was still ongoing".to_string()));
    }
    Ok(())
}*/

// ------------------------------------
// Support group related event handling
// ------------------------------------
/*
*/

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
        .field("Timestamp:", thread.incident_time, false)
        .field("Thread:", format!("<#{}>", thread.thread_id), false)
}
