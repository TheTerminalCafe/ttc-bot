use crate::{helper_functions::*, PostgresClientType, ThreadNameRegexType};
use serenity::{
    client::Context,
    framework::standard::{
        macros::{command, group},
        CommandError, CommandResult,
    },
    model::channel::Message,
    utils::Color,
};

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

    let mut data = ctx.data.write().await;

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

    let thread_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(thread_name_safe.clone())
                    .field("Description:", description, false)
                    .field("Incident:", incident, false)
                    .field("System info:", system_info, false)
                    .field(
                        "Attachments:",
                        attachments_msg
                            .attachments
                            .iter()
                            .map(|a| {
                                let mut url = a.url.clone();
                                url.push(' ');
                                url
                            })
                            .collect::<String>(),
                        false,
                    )
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

    let postgres_client = data.get_mut::<PostgresClientType>();

    Ok(())
}

#[command]
#[description("Close the current support thread")]
#[only_in(guilds)]
async fn solve(ctx: &Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write().await;

    Ok(())
}
