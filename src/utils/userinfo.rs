use crate::{
    traits::context_ext::ContextExt, traits::readable::Readable, utils::emoji_cache::EmojiCache,
    Context, Error,
};
use lazy_static::lazy_static;
use magick_rust::{
    bindings::{
        AlphaChannelOption_SetAlphaChannel, CompositeOperator_CopyCompositeOp,
        FilterType_LanczosFilter, MagickBooleanType_MagickTrue,
    },
    DrawingWand, MagickWand, PixelWand,
};
use poise::{
    serenity_prelude::{CreateEmbed, Emoji, User},
    CreateReply,
};
use regex::Regex;
use sqlx::{Pool, Postgres};
use std::{collections::HashMap, env::current_dir, fs, iter::Iterator, sync::atomic::AtomicBool};
use std::{io::Cursor, path::PathBuf};

// Image specific variables
const IMAGE_CACHE: &str = "image-cache";
// Usually the Emoji has no GET attributes but it's there just in case
const PATTERN_EXTENSION: &str = ".+\\.(.+?)(?:$|\\?)";
const IMAGE_OUTPUT_NAME: &str = "output.png";
const EMOJI_SIZE: u32 = 128;
const EMOJI_SPACING: u32 = 16;
const TEXT_SPACE: u32 = 150;
const FONT_SIZE: f64 = 52.0;

/// This should only be accessed to release the lock after the Image was sent
pub static IS_RUNNING: AtomicBool = AtomicBool::new(false);

pub async fn userinfo_fn<'a>(
    ctx: Context<'_>,
    user: User,
    emoji_stats: Option<&'a str>,
) -> Result<Option<CreateReply<'a>>, Error> {
    let mut reply = CreateReply::default();
    let mut embed = CreateEmbed::default();
    let color = ctx.data().colors.user_server_info().await;

    if emoji_stats.is_some() && EmojiCache::is_running() {
        ctx.send_simple(
            true,
            "The Emoji Cache isn't currently accessible",
            Some("Please try again later or without ``emoji_stats``"),
            ctx.data().colors.emoji_cache_inaccessible().await,
        )
        .await?;
        return Ok(None);
    }

    if emoji_stats.is_some() && ctx.guild().is_none() {
        ctx.send_simple(
            true,
            "You can't get the Emoji stats outside of the Guild",
            None,
            ctx.data().colors.emoji_cache_inaccessible().await,
        )
        .await?;
        return Ok(None);
    }

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

    if emoji_stats.is_some() {
        if IS_RUNNING.swap(true, std::sync::atomic::Ordering::Relaxed) {
            ctx.send_simple(
                true,
                "Another user is trying to generate an Image. Please try again in a few seconds",
                None,
                ctx.data().colors.general_error().await,
            )
            .await?;
            return Ok(None);
        }
        // ``ctx.guild()`` is checked above
        let emojis = ctx.guild().unwrap().emojis(ctx.discord()).await?;
        let mut emojis_hmap = HashMap::new();
        for emoji in emojis.clone() {
            emojis_hmap.insert(emoji.name.clone(), emoji);
        }
        let mut emoji_data = EmojiCache::new(&*ctx.data().pool)
            .get_data()
            .await?
            .user_emojis_vec();
        emoji_data.sort_by_key(|k| k.2);
        emoji_data.reverse();
        match download_emojis(emojis, &*ctx.data().pool).await {
            Ok(_) => (),
            Err(why) => {
                // Release lock in case of Error
                IS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
                return Err(why);
            }
        }
        let mut data_vec = Vec::new();
        for (userid, emoji_name, num) in emoji_data {
            if userid != user.id.0 {
                continue;
            }
            data_vec.push((get_filepath(&emoji_name, &*ctx.data().pool).await?, num));
        }
        if data_vec.len() == 0 {
            embed.field("Emoji stats", "There are no Emojis stats since the user didn't send Emojis yet or the Cache is too old", false);
        } else {
            match generate_userinfo_emoji_image(data_vec).await {
                Ok(_) => (),
                Err(why) => {
                    // Release lock in case of Error
                    IS_RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);
                    return Err(why);
                }
            }
            reply.attachment(emoji_stats.unwrap().into());
            embed.attachment(IMAGE_OUTPUT_NAME);
        }
    }

    reply.embeds.push(embed);
    reply.ephemeral(true);

    Ok(Some(reply))
}

struct Position {
    x: u32,
    y: u32,
}

// ----------------------
// Path related functions
// ----------------------

/// Get the path where the images are saved
pub fn get_basepath() -> Result<PathBuf, Error> {
    let mut basepath = current_dir()?;
    basepath.push(IMAGE_CACHE);
    Ok(basepath)
}

/// Get the filepath to a emoji image
async fn get_filepath(emoji: &String, pool: &Pool<Postgres>) -> Result<String, Error> {
    let data = sqlx::query!(
        "SELECT id, extension FROM ttc_emoji_download WHERE name = $1",
        emoji
    )
    .fetch_one(pool)
    .await?;
    let mut basepath = get_basepath()?;
    basepath.push(format!("{}.{}", data.id, data.extension));
    Ok(basepath.to_str().unwrap().to_string())
}

/// Gets the full path for the generated image
pub fn get_image_output_path() -> Result<String, Error> {
    let mut path = get_basepath()?;
    path.push(IMAGE_OUTPUT_NAME);
    Ok(path.to_str().unwrap().to_string())
}

// --------------------
// DB related functions
// --------------------

/// Inserts a Emoji in the Database and returns the ID for the Filename
async fn add_emoji_to_db_and_get_id(
    emoji: &String,
    extension: &String,
    pool: &Pool<Postgres>,
) -> Result<u32, Error> {
    Ok(sqlx::query!(
        r#"INSERT INTO ttc_emoji_download (name, extension) VALUES ($1, $2) RETURNING id"#,
        emoji,
        extension
    )
    .fetch_one(pool)
    .await?
    .id as u32)
}

/// Check if a emoji is already in the database/downloaded
async fn has_emoji(emoji: &String, pool: &Pool<Postgres>) -> Result<bool, Error> {
    if sqlx::query!(
        r#"SELECT COUNT(name) from ttc_emoji_download WHERE name = $1"#,
        emoji
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap()
        > 0
    {
        return Ok(true);
    }
    Ok(false)
}

// --------------
// Main functions
// --------------

/// Download all emojis that aren't currently downloaded
async fn download_emojis(emojis: Vec<Emoji>, pool: &Pool<Postgres>) -> Result<(), Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(PATTERN_EXTENSION).unwrap();
    };
    let basepath = get_basepath()?;
    fs::create_dir_all(basepath.clone())?;
    for emoji in emojis {
        if has_emoji(&emoji.name, pool).await? {
            continue;
        }
        let url = emoji.url();
        let extension = match RE.captures(&url) {
            Some(s) => match s.get(1) {
                Some(s) => s.as_str(),
                None => {
                    log::error!("Couldn't find extension for {}", &url);
                    return Err(Error::from("No extension found while downloading a Emoji"));
                }
            },
            None => {
                log::error!("Error matching Regex for {}", &url);
                return Err(Error::from("Error matching Regex for extension"));
            }
        };
        let filename = format!(
            "{}.{}",
            add_emoji_to_db_and_get_id(&emoji.name, &extension.to_string(), pool).await?,
            extension
        );
        let mut full_path = basepath.clone();
        full_path.push(filename);
        let response = reqwest::get(&url).await?;
        let mut file = std::fs::File::create(full_path.to_str().unwrap())?;
        let mut content = Cursor::new(response.bytes().await?);
        std::io::copy(&mut content, &mut file)?;
    }
    Ok(())
}

/// Generates the image for the userinfo
async fn generate_userinfo_emoji_image(values: Vec<(String, u64)>) -> Result<(), Error> {
    let mut pos = Position { x: 0, y: 0 };
    let mut num = values.len() as u32;
    if num >= 3 {
        num = 3;
    }
    let width = EMOJI_SIZE * num + (EMOJI_SPACING - 1) * num + TEXT_SPACE * num;
    let mut rows = values.len() / 3;
    if (values.len() % 3) != 0 {
        rows += 1;
    }
    let height = rows as u32 * (EMOJI_SIZE + EMOJI_SPACING) - EMOJI_SPACING;
    let mut count = 1;
    let mut mw_main_image = MagickWand::new();
    let mut dw_main_image = DrawingWand::new();
    let mut pw_main_image = PixelWand::new();
    let mut pw_font = PixelWand::new();
    let mut pw_font_outline = PixelWand::new();

    // Setup wands
    pw_main_image.set_color("none")?;
    pw_font.set_color("#ffffff")?;
    pw_font_outline.set_color("#000000")?;
    dw_main_image.set_fill_color(&pw_font);
    dw_main_image.set_stroke_color(&pw_font_outline);
    mw_main_image.new_image(width as usize, height as usize, &pw_main_image)?;
    mw_main_image.set_image_alpha_channel(AlphaChannelOption_SetAlphaChannel)?;
    dw_main_image.set_font("DejaVu Serif")?;
    dw_main_image.set_font_size(FONT_SIZE);
    dw_main_image.set_text_antialias(MagickBooleanType_MagickTrue);
    dw_main_image.set_stroke_antialias(MagickBooleanType_MagickTrue);
    dw_main_image.set_stroke_width(1.0);

    // Add images + text to the main image
    for image in values {
        let mw_tmp = MagickWand::new();
        mw_tmp.read_image(&image.0)?;
        let new_size = resize(mw_tmp.get_image_width(), mw_tmp.get_image_height());
        let offset = (
            (EMOJI_SIZE - new_size.0 as u32) / 2,
            (EMOJI_SIZE - new_size.1 as u32) / 2,
        );
        mw_tmp.resize_image(new_size.0, new_size.1, FilterType_LanczosFilter);
        mw_main_image.compose_images(
            &mw_tmp,
            CompositeOperator_CopyCompositeOp,
            false,
            (pos.x + offset.0) as isize,
            (pos.y + offset.1) as isize,
        )?;
        pos.x += EMOJI_SIZE + EMOJI_SPACING / 2;

        dw_main_image.draw_annotation(
            pos.x as f64,
            (pos.y + EMOJI_SIZE / 2 + (FONT_SIZE / 3.0) as u32) as f64,
            &image.1.to_string(),
        )?;
        pos.x += TEXT_SPACE;
        count += 1;
        if count > 3 {
            count = 1;
            pos.x = 0;
            pos.y += EMOJI_SIZE + EMOJI_SPACING;
        }
    }
    mw_main_image.draw_image(&dw_main_image)?;
    let output_path = get_image_output_path()?;
    mw_main_image.write_image(output_path.as_str())?;
    Ok(())
}

/// Scale the images while keeping the dimensions
fn resize(x: usize, y: usize) -> (usize, usize) {
    let mut bigger = x as f64;
    let mut smaller = y as f64;
    if y > x {
        bigger = y as f64;
        smaller = x as f64;
    }
    let divider: f64 = bigger as f64 / EMOJI_SIZE as f64;
    bigger /= divider;
    smaller /= divider;

    if x > y {
        (bigger as usize, smaller as usize)
    } else {
        (smaller as usize, bigger as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex() {
        lazy_static! {
            static ref RE: Regex = Regex::new(PATTERN_EXTENSION).unwrap();
        };
        let test_data = vec![
            ("https://cdn.discordapp.com/emojis/1234.webp", "webp"),
            (
                "https://cdn.discordapp.com/emojis/1234.webp?size=96&quality=lossless",
                "webp",
            ),
            ("https://cdn.discordapp.com/emojis/1234.png", "png"),
            (
                "https://cdn.discordapp.com/emojis/1234.png?quality=lossless",
                "png",
            ),
            ("https://cdn.discordapp.com/emojis/1234.123.png", "png"),
            (
                "https://cdn.discordapp.com/emojis/1234.123.png?size=96&quality=lossless",
                "png",
            ),
        ];
        for test in test_data {
            assert_eq!(
                RE.captures(test.0).unwrap().get(1).unwrap().as_str(),
                test.1
            );
        }
    }
}
