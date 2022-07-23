use std::{collections::HashMap, time::Instant};

use poise::serenity_prelude::{ChannelId, Color, Message, RwLock, UserId, Webhook};
use sqlx::PgPool;

use crate::{
    config_function, ttc_embed_color,
    utils::bee_utils::{BeeifiedUser, BeezoneChannel},
};

pub struct Data {
    pub users_currently_questioned: RwLock<Vec<UserId>>,
    pub harold_message: RwLock<Option<Message>>,
    pub beeified_users: RwLock<HashMap<UserId, BeeifiedUser>>,
    pub beezone_channels: RwLock<HashMap<ChannelId, BeezoneChannel>>,
    pub webhooks: RwLock<HashMap<ChannelId, Webhook>>,
    pub pool: PgPool,
    pub thread_name_regex: regex::Regex,
    pub startup_time: Instant,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Implementations of the config keys in the database using the macro to reduce code duplication
impl Data {
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.support_channel as support_channel
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        support_channel,
        i64
    );
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.welcome_channel as welcome_channel
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        welcome_channel,
        i64
    );
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.verified_role as verified_role
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        verified_role,
        i64
    );
    config_function!(
        r#"select distinct
        tcp.id as config_properties_id,
        tcp.moderator_role as moderator_role
        from ttc_config tc
        inner join ttc_config_properties tcp on tc.config_properties_id = tcp.id"#,
        moderator_role,
        i64
    );
    config_function!(
        r#"select distinct
        tcbc.id as conveyance_blacklist_id,
        tcbc.channel_id as conveyance_blacklist_channel
        from ttc_config tc
        inner join ttc_conveyance_blacklist_channel tcbc on tc.conveyance_blacklist_id  = tcbc.id order by tcbc.id asc"#,
        conveyance_blacklist_channel,
        Vec<i64>
    );
    config_function!(
        r#"select distinct
        tcc.id as conveyance_id,
        tcc.channel_id as conveyance_channel
        from ttc_config tc
        inner join ttc_conveyance_channel tcc on tc.conveyance_id = tcc.id order by tcc.id asc"#,
        conveyance_channel,
        Vec<i64>
    );
    config_function!(
        r#"select distinct
        the.id as harold_emoji_id,
        the."name" as harold_emoji
        from ttc_config tc
        inner join ttc_harold_emoji the on tc.harold_emoji_id = the.id order by the.id asc"#,
        harold_emoji,
        Vec<String>
    );
    config_function!(
        r#"select distinct
        twm.id as welcome_message_id,
        twm.welcome_message as welcome_message
        from ttc_config tc
        inner join ttc_welcome_message twm on tc.welcome_message_id = twm.id order by twm.id asc"#,
        welcome_message,
        Vec<String>
    );
    config_function!(
        r#"select role_id as selfroles, emoji_name
        from ttc_selfroles"#,
        selfroles,
        emoji_name,
        Vec<(i64, Option<String>)>
    );
    pub async fn get_embed_color(&self, embed_type: &str) -> Result<Color, Error> {
        let data = sqlx::query!(
            r#"SELECT color FROM ttc_embed_colors WHERE embed_type = $1"#,
            embed_type
        )
        .fetch_one(&self.pool)
        .await?
        .color;
        if data.len() < 3 {
            return Err(Error::from("Not enough color bytes in Database"));
        }
        return Ok(Color::from_rgb(data[0], data[1], data[2]));
    }

    // General
    ttc_embed_color!(verification_message, "verification_message", Color::FOOYOO);
    ttc_embed_color!(ping, "ping", Color::BLUE);
    ttc_embed_color!(help, "help", Color::FOOYOO);
    ttc_embed_color!(user_server_info, "user_server_info", Color::BLITZ_BLUE);
    ttc_embed_color!(translate, "translate", Color::FOOYOO);
    ttc_embed_color!(support_info, "support_info", Color::FOOYOO);

    // Moderation + Admin
    ttc_embed_color!(admin_success, "admin_success", Color::FOOYOO);
    ttc_embed_color!(mod_success, "mod_success", Color::FOOYOO);
    ttc_embed_color!(mod_punish, "mod_punish", Color::RED);

    // Conveyance
    ttc_embed_color!(conveyance_msg_delete, "conveyance_msg_delete", Color::GOLD);
    ttc_embed_color!(
        conveyance_msg_update,
        "conveyance_msg_update",
        Color::DARK_GOLD
    );
    ttc_embed_color!(
        conveyance_member_join,
        "conveyance_member_join",
        Color::FOOYOO
    );
    ttc_embed_color!(
        conveyance_member_leave,
        "conveyance_member_leave",
        Color::RED
    );
    ttc_embed_color!(
        conveyance_member_update,
        "conveyance_member_update",
        Color::ORANGE
    );
    ttc_embed_color!(
        conveyance_ban_addition,
        "conveyance_ban_addition",
        Color::DARK_RED
    );
    ttc_embed_color!(conveyance_unban, "conveyance_unban", Color::FOOYOO);

    // Interactions
    ttc_embed_color!(verify_color, "verify_color", Color::FOOYOO);
    ttc_embed_color!(selfrole_selection, "selfrole_selection", Color::PURPLE);
    ttc_embed_color!(
        selfrole_post_edit_msg,
        "selfrole_post_edit_msg",
        Color::FOOYOO
    );
    ttc_embed_color!(
        ticket_has_already_ticket,
        "ticket_has_already_ticket",
        Color::PURPLE
    );
    ttc_embed_color!(
        ticket_thread_created,
        "ticket_thread_created",
        Color::FOOYOO
    );
    ttc_embed_color!(ticket_summary, "ticket_summary", Color::FOOYOO);

    // Leaderboard
    ttc_embed_color!(
        leaderboard_harold_leaderboard,
        "leaderboard_harold_leaderboard",
        Color::FOOYOO
    );
    ttc_embed_color!(
        leaderboard_message_count_leaderboard,
        "leaderboard_message_count_leaderboard",
        Color::BLUE
    );
    ttc_embed_color!(
        leaderboard_harold_percentage_leaderboard,
        "leaderboard_harold_percentage_leaderboard",
        Color::PURPLE
    );
    ttc_embed_color!(leaderboard_global, "leaderboard_global", Color::DARK_GOLD);
    ttc_embed_color!(
        leaderboard_user_overview,
        "leaderboard_user_overview",
        Color::BLURPLE
    );

    // Other
    ttc_embed_color!(input_error, "input_error", Color::RED);
    ttc_embed_color!(input_warn, "input_warn", Color::ORANGE);
    ttc_embed_color!(general_error, "general_error", Color::RED);
    ttc_embed_color!(bump_message, "bump_message", Color::PURPLE);
    ttc_embed_color!(emoji_info, "emoji_info", Color::FOOYOO);
    ttc_embed_color!(bee_translate_block, "bee_translate_block", Color::KERBAL);
}

/*#[derive(Debug, Clone)]
pub struct Config {
    pub support_channel: i64,
    pub conveyance_channels: Vec<i64>,
    pub conveyance_blacklisted_channels: Vec<i64>,
    pub welcome_channel: i64,
    pub verified_role: i64,
    pub moderator_role: i64,
    pub welcome_messages: Vec<String>,
}

impl Config {
    pub async fn save_in_db(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM ttc_config"#)
            .execute(pool)
            .await?;

        sqlx::query!(
            r#"INSERT INTO ttc_config VALUES($1, $2, $3, $4, $5, $6, $7)"#,
            self.support_channel,
            &self.conveyance_channels,
            &self.conveyance_blacklisted_channels,
            self.welcome_channel,
            self.verified_role,
            self.moderator_role,
            &self.welcome_messages,
        )
        .execute(pool)
        .await?;

        log::info!("Settings saved.");

        Ok(())
    }

    pub async fn get_from_db(pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(Self, r#"SELECT * FROM ttc_config"#)
            .fetch_one(pool)
            .await
    }
}*/
