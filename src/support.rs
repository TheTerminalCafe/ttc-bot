use crate::{helper_functions::*, PgPoolType, ThreadNameRegexType, UsersCurrentlyQuestionedType};
use chrono::{DateTime, Utc};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        CommandError, CommandResult,
    },
    model::channel::Message,
    utils::Color,
};

// Support thread related types
#[derive(Debug)]
struct SupportThread {
    incident_id: i32,
    thread_id: i64,
    incident_time: DateTime<Utc>,
    incident_status: String,
    user_id: i64,
    incident_title: String,
    incident_type: String,
}

// Group creation

#[group]
#[prefixes("support")]
#[description("Support related commands")]
#[commands(new, solve)]
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
    wait_for_message(ctx, msg).await?;

    // Ask for the details of the issue
    // The loops are for making sure there is at least some text content in the message
    embed_msg(ctx, msg, "**Title?**", Color::BLUE).await?;
    let thread_name_msg = loop {
        let new_msg = wait_for_message(ctx, msg).await?;
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
        let new_msg = wait_for_message(ctx, msg).await?;
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
        let new_msg = wait_for_message(ctx, msg).await?;
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
        let new_msg = wait_for_message(ctx, msg).await?;
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
        r#"INSERT INTO ttc_support_tickets (thread_id, incident_time, incident_status, user_id, incident_title, incident_type) VALUES($1, $2, $3, $4, $5, $6) RETURNING *"#,
        thread_id.0 as i64,
        Utc::now(),
        "active",
        msg.author.id.0 as i64,
        thread_name,
        "placeholder",
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let new_thread_name = format!("[{}] {}", thread.incident_id, thread_name);

    thread_id
        .edit_thread(ctx, |t| t.name(&new_thread_name))
        .await?;

    sqlx::query!(
        r#"UPDATE ttc_support_tickets SET incident_title = $2 WHERE thread_id = $1"#,
        msg.channel_id.0 as i64,
        new_thread_name,
    )
    .execute(pool)
    .await
    .unwrap();

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

    if thread.incident_status != "active" {
        embed_msg(ctx, msg, "**Error**: Thread already solved", Color::RED).await?;
    }

    embed_msg(ctx, msg, "**Great!**\n\nNow that the issue is solved it is time to give back to the society. Send the details of the solution after this message.", Color::FOOYOO).await?;

    wait_for_message(ctx, msg).await?;

    // Archive the thread after getting the solution
    msg.channel_id
        .edit_thread(ctx, |t| {
            t.name(format!("[SOLVED] {}", thread.incident_title))
                .archived(true)
        })
        .await?;

    // Update the state to be archived
    sqlx::query!(
        r#"UPDATE ttc_support_tickets SET incident_status = 'archived' WHERE thread_id = $1"#,
        msg.channel_id.0 as i64
    )
    .execute(pool)
    .await
    .unwrap();

    Ok(())
}
