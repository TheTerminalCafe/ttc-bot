use poise::serenity_prelude::Color;
use sqlx::PgPool;
use std::sync::Arc;

macro_rules! embed_color {
    ($name:ident, $default_color:expr) => {
        pub async fn $name(&self) -> ::poise::serenity_prelude::Color {
            let data = match sqlx::query!(
                r#"SELECT color FROM ttc_embed_colors WHERE embed_type = $1"#,
                stringify!($name)
            )
            .fetch_one(&*self.pool)
            .await
            {
                Ok(c) => Ok(c.color),
                Err(why) => Err(why),
            };
            match data {
                Ok(data) => {
                    if data.len() >= 3 {
                        return ::poise::serenity_prelude::Color::from_rgb(
                            data[0], data[1], data[2],
                        );
                    } else {
                        ::log::warn!(
                            "Not enough color bytes in Database for color {}",
                            stringify!($name)
                        );
                        return $default_color;
                    }
                }
                Err(why) => {
                    match why {
                        ::sqlx::Error::RowNotFound => {
                            ::log::warn!(
                                "No color set in Database for \"{}\": {}",
                                stringify!($name),
                                why
                            );
                        }
                        _ => ::log::error!(
                            "Error getting color \"{}\" for reply: {}",
                            stringify!($name),
                            why
                        ),
                    }
                    $default_color
                }
            }
        }
    };
}

pub struct Colors {
    pool: Arc<PgPool>,
}

impl Colors {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    // General
    embed_color!(verification_message, Color::FOOYOO);
    embed_color!(ping, Color::BLUE);
    embed_color!(help, Color::FOOYOO);
    embed_color!(user_server_info, Color::BLITZ_BLUE);
    embed_color!(translate, Color::FOOYOO);
    embed_color!(support_info, Color::FOOYOO);

    // Moderation + Admin
    embed_color!(admin_success, Color::FOOYOO);
    embed_color!(mod_success, Color::FOOYOO);
    embed_color!(mod_punish, Color::RED);

    // Conveyance
    embed_color!(conveyance_msg_delete, Color::GOLD);
    embed_color!(conveyance_msg_update, Color::DARK_GOLD);
    embed_color!(conveyance_member_join, Color::FOOYOO);
    embed_color!(conveyance_member_leave, Color::RED);
    embed_color!(conveyance_member_update, Color::ORANGE);
    embed_color!(conveyance_ban_addition, Color::DARK_RED);
    embed_color!(conveyance_unban, Color::FOOYOO);

    // Interactions
    embed_color!(verify_color, Color::FOOYOO);
    embed_color!(selfrole_selection, Color::PURPLE);
    embed_color!(selfrole_post_edit_msg, Color::FOOYOO);
    embed_color!(ticket_has_already_ticket, Color::PURPLE);
    embed_color!(ticket_thread_created, Color::FOOYOO);
    embed_color!(ticket_summary, Color::FOOYOO);

    // Leaderboard
    embed_color!(leaderboard_harold_leaderboard, Color::FOOYOO);
    embed_color!(leaderboard_message_count_leaderboard, Color::BLUE);
    embed_color!(leaderboard_harold_percentage_leaderboard, Color::PURPLE);
    embed_color!(leaderboard_global, Color::DARK_GOLD);
    embed_color!(leaderboard_user_overview, Color::BLURPLE);

    // Other
    embed_color!(input_error, Color::RED);
    embed_color!(input_warn, Color::ORANGE);
    embed_color!(general_error, Color::RED);
    embed_color!(bump_message, Color::PURPLE);
    embed_color!(emoji_info, Color::FOOYOO);
    embed_color!(bee_translate_block, Color::KERBAL);
}
