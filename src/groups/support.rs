use std::time::Duration;

use crate::{
    typemap::{
        config::Config,
        types::{PgPoolType, ThreadNameRegexType, UsersCurrentlyQuestionedType},
    },
    utils::helper_functions::*,
};
use chrono::{DateTime, Utc};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{check, command, group},
        Args, CommandError, CommandOptions, CommandResult, Reason,
    },
    model::{
        channel::{GuildChannel, Message},
        id::UserId,
    },
    utils::Color,
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

// Group creation

#[group]
#[prefixes("support")]
#[only_in(guilds)]
#[description("Support related commands")]
#[commands(new, solve, search, list)]
#[default_command(new)]
struct Support;

// ----------------------
// Support group commands
// ----------------------

#[command]
#[description("Create a new support thread")]
#[checks(is_in_support_channel)]
#[num_args(0)]
async fn new(ctx: &Context, msg: &Message) -> CommandResult {
    // Make sure the write lock to data isn't held for the entirety of this command. This causes
    // the code to be a bit messier but concurrency has forced my hand
    {
        let mut data = ctx.data.write().await; // Get a writeable reference to the data
        let users_currently_questioned = data.get_mut::<UsersCurrentlyQuestionedType>().unwrap();

        if users_currently_questioned.contains(&msg.author.id) {
            return Err(CommandError::from("User already being questioned!"));
        }
        users_currently_questioned.push(msg.author.id);
    }

    // Send a summary message
    let info_msg = msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| { e.title("Support ticket creation")
            .description("You will be asked for the following fields during the ticket creation.")
            .field("Title:", "The title for this issue.", false)
            .field("Description:", "A more in depth explanation of the issue.", false)
            .field("Incident:", "Anything that could have caused this issue in the first place.", false)
            .field("System info:", "Information about your system. OS, OS version, hardware, any info about hardware/software possibly related to this issue.", false)
            .field("Attachments:", "Any attachments related to the issue. If the message contains no attachments none will be shown", false)
            .color(Color::PURPLE)})
    }).await?;

    // Ask for the details of the issue
    let thread_name = get_message_reply(
        ctx,
        msg,
        |m| {
            m.embed(|e| {
                e.description("**Title?** (300 seconds time limit, ~100 character limit)")
                    .color(Color::BLUE)
            })
        },
        Duration::from_secs(300),
    )
    .await?;

    // Parse the thread name with the regex to avoid special characters in thread name
    let mut thread_name_safe = {
        let data = ctx.data.read().await;
        data.get::<ThreadNameRegexType>()
            .unwrap()
            .replace_all(&thread_name, "")
            .to_string()
    };

    let mut description = get_message_reply(
        ctx,
        msg,
        |m| {
            m.embed(|e| {
                e.description("**Description?** (300 seconds time limit, 1024 character limit)")
                    .color(Color::BLUE)
            })
        },
        Duration::from_secs(300),
    )
    .await?;

    let mut incident = get_message_reply(
        ctx,
        msg,
        |m| {
            m.embed(|e| {
                e.description("**Incident?** (300 seconds time limit, 1024 character limit)")
                    .color(Color::BLUE)
            })
        },
        Duration::from_secs(300),
    )
    .await?;

    let mut system_info = get_message_reply(
        ctx,
        msg,
        |m| {
            m.embed(|e| {
                e.description("**System info?** (300 seconds time limit, 1024 character limit)")
                    .color(Color::BLUE)
            })
        },
        Duration::from_secs(300),
    )
    .await?;

    let att_msg = embed_msg(
        ctx,
        &msg.channel_id,
        None,
        Some("**Attachments?** (300 seconds time limit)"),
        Some(Color::BLUE),
        None,
    )
    .await?;

    // The helper function cant really be used for the attachment messages due to much of the
    // checking it does
    let attachments_msg = wait_for_message(ctx, msg, Duration::from_secs(300)).await?;
    // Make sure all attachments with image types get added as images to the embed
    let mut image_attachments = attachments_msg.attachments.clone();
    image_attachments.retain(|a| {
        if let Some(ct) = &a.content_type {
            return ct.contains("image");
        }
        false
    });

    // Get the string of the urls to the attachments
    let mut attachments_str = attachments_msg
        .attachments
        .iter()
        .map(|a| {
            let mut url = a.url.clone();
            url.push(' ');
            url
        })
        .collect::<String>();
    if attachments_str == "" {
        attachments_str = "None".to_string();
    }

    match msg
        .channel_id
        .delete_messages(ctx, vec![attachments_msg.id, att_msg.id, info_msg.id])
        .await
    {
        Ok(_) => (),
        Err(why) => log::error!("Error deleting messages: {}", why),
    }

    // Finally remove the user id from the currently questioned list to allow them to run
    // ttc!support new again
    {
        let mut data = ctx.data.write().await;
        data.get_mut::<UsersCurrentlyQuestionedType>()
            .unwrap()
            .retain(|uid| uid != &msg.author.id);
    }
    // Truncate the strings to match the character limits of the embed
    thread_name_safe.truncate(100);
    description.truncate(1024);
    system_info.truncate(1024);
    incident.truncate(1024);
    attachments_str.truncate(1024);

    // Get the author name to use on the embed
    let author_name = msg
        .author_nick(ctx)
        .await
        .unwrap_or(msg.author.name.clone());

    // The message to start the support thread containing all the given information
    let thread_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(thread_name_safe.clone())
                    .description("Once the issue has been solved, please use the command `ttc!support solve` to mark the thread as such.")
                    .author(|a| a.name(author_name).icon_url(msg.author.face()))
                    .field("Description:", description, false)
                    .field("Incident:", incident, false)
                    .field("System info:", system_info, false)
                    .field("Attachments:", attachments_str, false)
                    .color(Color::FOOYOO);
                for attachment in &image_attachments {
                    e.image(&attachment.url);
                }
                e
            })
        })
        .await?;

    // Here the data variable doesn't live long and a read lock is much better for smooth
    // operation, so it can be locked "globally" like this
    let data = ctx.data.read().await;

    let pool = data.get::<PgPoolType>().unwrap();
    // Select auto archive duration based on the server boost level
    let thread_id = msg
        .channel_id
        .create_public_thread(ctx, thread_msg.id, |ct| ct.name(thread_name_safe.clone()))
        .await?
        .id;

    // Insert the gathered information into the database and return the newly created database
    // entry for it's primary key to be added to the support thread title
    let thread = match sqlx::query_as!(
        SupportThread,
        r#"INSERT INTO ttc_support_tickets (thread_id, user_id, incident_time, incident_title, incident_solved) VALUES($1, $2, $3, $4, $5) RETURNING *"#,
        thread_id.0 as i64,
        msg.author.id.0 as i64,
        Utc::now(),
        thread_name_safe,
        false,
    )
    .fetch_one(pool)
    .await {
        Ok(thread) => thread,
        Err(why) => {
            return Err(CommandError::from(format!("Error writing into database: {}", why)));
        }
    };

    let mut new_thread_name = format!("[{}] {}", thread.incident_id, thread_name_safe);
    new_thread_name.truncate(100);

    thread_id
        .edit_thread(ctx, |t| t.name(&new_thread_name))
        .await?;

    Ok(())
}

#[command]
#[description("Solve the current support thread")]
#[checks(is_in_support_thread)]
#[num_args(0)]
async fn solve(ctx: &Context, msg: &Message) -> CommandResult {
    // Get a reference to the database
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    // Get the row with the current channel id from the database
    let thread = match sqlx::query_as!(
        SupportThread,
        r#"SELECT * FROM ttc_support_tickets WHERE thread_id = $1"#,
        msg.channel_id.0 as i64
    )
    .fetch_one(pool)
    .await
    {
        Ok(thread) => thread,
        Err(why) => {
            return Err(CommandError::from(format!(
                "Unable to read from database: {}",
                why
            )));
        }
    };

    // Make sure thread is not yet solved
    if thread.incident_solved {
        embed_msg(
            ctx,
            &msg.channel_id,
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
        msg.channel_id.0 as i64
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(why) => {
            return Err(CommandError::from(format!(
                "Error reading from database: {}",
                why
            )));
        }
    }

    embed_msg(
        ctx,
        &msg.channel_id,
        Some("Great!"),
        Some("Now that the issue is solved, you can give back to society and send the solution after this message."),
        Some(Color::FOOYOO),
        None
        )
        .await?;

    match wait_for_message(ctx, msg, Duration::from_secs(300)).await {
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
    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();
    let support_channel_id = match Config::get_from_db(pool).await {
        Ok(config) => config.support_channel,
        Err(why) => {
            return Err(Reason::Log(format!(
                "Getting config from database failed: {}",
                why
            )))
        }
    } as u64;

    if support_channel_id != msg.channel_id.0 {
        return Err(Reason::Log(format!(
            "{} called outside support channel",
            msg.content
        )));
    }

    Ok(())
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

// ------------------------------------
// Support group related event handling
// ------------------------------------

pub async fn thread_update(ctx: &Context, thread: &GuildChannel) {
    // Make sure the updated part is the archived value
    if thread.thread_metadata.unwrap().archived {
        let data = ctx.data.read().await;
        let pool = data.get::<PgPoolType>().unwrap();

        // Get the current thread info from the database
        let db_thread = match sqlx::query_as!(
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

            // Inform the author of the issue about the unarchival
            match embed_msg(
                ctx,
                &thread.id,
                Some("Thread unarchived"),
                Some("If the issue has already been solved make sure to mark it as such with `ttc!support solve`"),
                None,
                None
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
