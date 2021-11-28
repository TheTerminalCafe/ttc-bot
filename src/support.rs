use crate::{helper_functions::*, PgPoolType, ThreadNameRegexType, UsersCurrentlyQuestionedType};
use chrono::{DateTime, Utc};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        Args, CommandError, CommandResult,
    },
    model::channel::Message,
    utils::Color,
};

// Support thread related types
#[derive(Debug)]
pub struct SupportThread {
    pub incident_id: i32,
    pub thread_id: i64,
    pub user_id: i64,
    pub incident_time: DateTime<Utc>,
    pub incident_title: String,
    pub thread_archived: bool,
    pub incident_solved: bool,
}

// Group creation

#[group]
#[prefixes("support")]
#[description("Support related commands")]
#[commands(new, solve, search)]
struct Support;

// ----------------------
// Support group commands
// ----------------------

#[command]
#[description("Create a new support thread")]
#[only_in(guilds)]
async fn new(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut users_currently_questioned = data.get_mut::<UsersCurrentlyQuestionedType>().unwrap();

    if users_currently_questioned.contains(&msg.author.id) {
        return Err(CommandError::from("User already being questioned!"));
    }
    users_currently_questioned.push(msg.author.id);

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|e| { e.title("Support ticket creation")
            .description("You will be asked for the following fields after you send anything after this message.")
            .field("Title:", "The title for this issue.", false)
            .field("Description:", "A more in depth explanation of the issue.", false)
            .field("Incident:", "Anything that could have caused this issue in the first place.", false)
            .field("System info:", "Information about your system. OS, OS version, hardware, any info about hardware/software possibly related to this issue.", false)
            .field("Attachments:", "Any attachments related to the issue. If the message contains no attachments none will be shown", false)
            .color(Color::PURPLE)})
    }).await?;

    // Wait for acknowledgement message
    if let Err(_) = wait_for_message(ctx, msg).await {
        users_currently_questioned.retain(|uid| uid != &msg.author.id);
        return Err(CommandError::from("User took too long to respond"));
    }

    // Ask for the details of the issue
    // The loops are for making sure there is at least some text content in the message
    embed_msg(ctx, msg, "**Title?**", Color::BLUE).await?;
    let thread_name_msg = loop {
        let new_msg = match wait_for_message(ctx, msg).await {
            Ok(msg) => msg,
            Err(_) => {
                users_currently_questioned.retain(|uid| uid != &msg.author.id);
                return Err(CommandError::from("User took too long to respond"));
            }
        };
        if new_msg.content_safe(ctx).await != "" {
            break new_msg;
        }
        embed_msg(
            ctx,
            msg,
            "Please send a message with text content.",
            Color::RED,
        )
        .await?;
    };

    embed_msg(ctx, msg, "**Description?**", Color::BLUE).await?;
    let description_msg = loop {
        let new_msg = match wait_for_message(ctx, msg).await {
            Ok(msg) => msg,
            Err(_) => {
                users_currently_questioned.retain(|uid| uid != &msg.author.id);
                return Err(CommandError::from("User took too long to respond"));
            }
        };
        if new_msg.content != "" {
            break new_msg;
        }
        embed_msg(
            ctx,
            msg,
            "Please send a message with text content.",
            Color::RED,
        )
        .await?;
    };

    embed_msg(ctx, msg, "**Incident?**", Color::BLUE).await?;
    let incident_msg = loop {
        let new_msg = match wait_for_message(ctx, msg).await {
            Ok(msg) => msg,
            Err(_) => {
                users_currently_questioned.retain(|uid| uid != &msg.author.id);
                return Err(CommandError::from("User took too long to respond"));
            }
        };
        if new_msg.content != "" {
            break new_msg;
        }
        embed_msg(
            ctx,
            msg,
            "Please send a message with text content.",
            Color::RED,
        )
        .await?;
    };

    embed_msg(ctx, msg, "**System info?**", Color::BLUE).await?;

    let system_info_msg = loop {
        let new_msg = match wait_for_message(ctx, msg).await {
            Ok(msg) => msg,
            Err(_) => {
                users_currently_questioned.retain(|uid| uid != &msg.author.id);
                return Err(CommandError::from("User took too long to respond"));
            }
        };
        if new_msg.content != "" {
            break new_msg;
        }
        embed_msg(
            ctx,
            msg,
            "Please send a message with text content.",
            Color::RED,
        )
        .await?;
    };

    embed_msg(ctx, msg, "**Attachments?**", Color::BLUE).await?;

    let attachments_msg = wait_for_message(ctx, msg).await?;

    users_currently_questioned.retain(|uid| uid != &msg.author.id);

    // Get the precompiled regex from data
    let re = match data.get::<ThreadNameRegexType>() {
        Some(re) => re,
        None => return Err(CommandError::from("No thread name regex!")),
    };

    // The content_safe makes sure there are no pings or stuff like that in the text
    let thread_name = thread_name_msg.content_safe(ctx).await;
    let thread_name_safe = re.replace_all(&thread_name, ""); // Parse the thread name with the regex to avoid special characters in thread name
    let description = description_msg.content_safe(ctx).await;
    let system_info = system_info_msg.content_safe(ctx).await;
    let incident = incident_msg.content_safe(ctx).await;

    // Make sure all attachments with image types get added as images to the embed
    let mut image_attachments = attachments_msg.attachments.clone();
    image_attachments.retain(|a| {
        if let Some(ct) = &a.content_type {
            return ct.contains("image");
        }
        false
    });

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

    let thread_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(thread_name_safe.clone())
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

    let thread_id = msg
        .channel_id
        .create_public_thread(ctx, thread_msg.id, |ct| ct.name(thread_name_safe))
        .await?
        .id;

    let pool = data.get::<PgPoolType>().unwrap();

    let thread = sqlx::query_as!(
        SupportThread,
        r#"INSERT INTO ttc_support_tickets (thread_id, user_id, incident_time, incident_title, thread_archived, incident_solved) VALUES($1, $2, $3, $4, $5, $6) RETURNING *"#,
        thread_id.0 as i64,
        msg.author.id.0 as i64,
        Utc::now(),
        thread_name,
        false,
        false,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let new_thread_name = format!("[{}] {}", thread.incident_id, thread_name);

    thread_id
        .edit_thread(ctx, |t| t.name(&new_thread_name))
        .await?;

    Ok(())
}

#[command]
#[description("Close the current support thread")]
#[only_in(guilds)]
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
            embed_msg(ctx, msg, "**Error**: Not in a support thread", Color::RED).await?;
            return Err(CommandError::from(why));
        }
    };

    if thread.incident_solved {
        embed_msg(ctx, msg, "**Error**: Thread already solved", Color::RED).await?;
    }

    embed_msg(ctx, msg, "**Great!**\n\nNow that the issue is solved it is time to give back to the society. Send the details of the solution after this message.", Color::FOOYOO).await?;

    wait_for_message(ctx, msg).await?;

    // Archive the thread after getting the solution
    msg.channel_id
        .edit_thread(ctx, |t| {
            t.name(format!(
                "[SOLVED] [{}] {}",
                thread.incident_id, thread.incident_title
            ))
            .archived(true)
        })
        .await?;

    // Update the state to be archived
    sqlx::query!(
        r#"UPDATE ttc_support_tickets SET thread_archived = 't', incident_solved = 't' WHERE thread_id = $1"#,
        msg.channel_id.0 as i64
    )
    .execute(pool)
    .await
    .unwrap();

    Ok(())
}

#[command]
#[sub_commands(id, title)]
async fn search(ctx: &Context, msg: &Message) -> CommandResult {
    embed_msg(
        ctx,
        msg,
        "Use search with one of the subcommands.",
        Color::RED,
    )
    .await?;

    Ok(())
}

#[command]
async fn title(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    args.quoted();

    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

    let mut was_found = false;

    for _ in 0..args.len() {
        let arg = match args.single::<String>() {
            Ok(arg) => arg,
            Err(why) => {
                embed_msg(
                    ctx,
                    msg,
                    &format!("Unable to parse argument: {}", why),
                    Color::RED,
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
            support_ticket_msg(ctx, msg, thread).await?;
            was_found = true;
        }
    }

    if !was_found {
        embed_msg(ctx, msg, "No support ticket found.", Color::RED).await?;
    }

    Ok(())
}

#[command]
async fn id(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let id = match args.single::<u32>() {
        Ok(id) => id,
        Err(why) => {
            embed_msg(
                ctx,
                msg,
                &format!("**Error**: Unable to parse provided ID: {}", why),
                Color::RED,
            )
            .await?;
            return Err(CommandError::from("Unable to parse provided ID"));
        }
    };

    let data = ctx.data.read().await;
    let pool = data.get::<PgPoolType>().unwrap();

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
                msg,
                &format!("No support ticket found for id [{}]", id),
                Color::RED,
            )
            .await?;
            return Err(CommandError::from(
                "No support ticket found for specified id",
            ));
        }
    };

    support_ticket_msg(ctx, msg, &thread);

    Ok(())
}
