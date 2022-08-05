use crate::{traits::readable::Readable, Context, Error};
use poise::{
    serenity_prelude::{CreateEmbed, User},
    CreateReply,
};

pub async fn userinfo_fn(ctx: Context<'_>, user: User) -> Result<Option<CreateReply>, Error> {
    let mut reply = CreateReply::default();
    let mut embed = CreateEmbed::default();
    let color = ctx.data().colors.user_server_info().await;

    let (nickname, joined_at, roles) = match ctx.guild() {
        Some(guild) => {
            match guild.member(ctx.discord(), user.id).await {
                Ok(member) => {
                    let nick = member.nick.clone().unwrap_or("None".to_string());
                    let joined_at = match member.joined_at {
                        Some(joined_at) => joined_at.readable(),
                        None => "N/A".to_string(),
                    };
                    let mut roles = match member.roles(ctx.discord()) {
                        Some(roles) => roles
                            .iter()
                            .map(|role| format!("<@&{}>, ", role.id))
                            .collect::<String>(),
                        None => "None".to_string(),
                    };
                    // Remove trailing comma and space
                    roles.pop();
                    roles.pop();

                    // Make sure it isn't empty
                    if roles == "" {
                        roles = "None".to_string()
                    }

                    (nick, joined_at, roles)
                }
                Err(_) => ("N/A".to_string(), "N/A".to_string(), "N/A".to_string()),
            }
        }
        None => ("N/A".to_string(), "N/A".to_string(), "N/A".to_string()),
    };

    let mut easter_egg_fields = Vec::new();
    if ctx.framework().bot_id.0 == user.id.0 {
        let data = sqlx::query!(r#"SELECT field_name, field_value FROM ttc_easter_egg_botinfo"#)
            .fetch_all(&*ctx.data().pool)
            .await?;
        for row in data {
            easter_egg_fields.push((row.field_name, row.field_value, false));
        }
    }

    embed
        .author(|a| a.name(user.tag()).icon_url(user.face()))
        .field("User ID", user.id.0, true)
        .field("Nickname", nickname, true)
        .field("Created At", user.id.created_at().readable(), false)
        .field("Joined At", joined_at, false)
        .field("Roles", roles, false)
        .field("Icon URL", user.face(), false)
        .fields(easter_egg_fields)
        .color(color);

    reply.embeds.push(embed);
    reply.ephemeral(true);

    Ok(Some(reply))
}
