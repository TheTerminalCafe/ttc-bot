use crate::{
    command_raw_error,
    traits::context_ext::ContextExt,
    utils::{autocomplete_functions::language_autocomplete, bee_utils},
    Context, Error,
};
use poise::serenity_prelude::Message;
use serde_json::Value;

mod language_codes;

pub use language_codes::LANGUAGE_CODES;


pub fn langcode_to_lang(code: &str) -> Option<&'static str> {
    LANGUAGE_CODES.iter()
        .filter(|(l, _)| l == &code)
        .next()
        .map(|(_, l)| l)
        .copied()
}

/// Translation command
///
/// Translates the provided text into the specified language.
/// ``translate [language] [text]``
#[poise::command(slash_command, prefix_command, category = "Localisation")]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "Target language"]
    #[autocomplete = "language_autocomplete"]
    lang: String,
    #[description = "The text to translate"]
    #[rest]
    text_to_translate: String,
) -> Result<(), Error> {

    if bee_apartheid(&ctx).await? {
        return Ok(());
    }
    // Get the language code and the text to translate

    if text_to_translate.is_empty() {
        empty_warning(&ctx).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let (source_lang, translated_text) = translate_text(&lang, &text_to_translate).await?;

    if !check_translated_length(&ctx, translated_text.len()).await? {
        return Ok(());
    }

    let color = ctx.data().colors.translate().await;

    // Send the translated message
    ctx.send_embed(false, |e| {
        e.title("Translated Message")
            .description(format!(
                "{} -> {}",
                langcode_to_lang(source_lang).unwrap_or(source_lang),
                langcode_to_lang(&lang).unwrap_or(&lang),
            ))
            .field("Original Message", &text_to_translate, false)
            .field("Translated Message", &translated_text, false)
            .color(color)
    })
    .await?;

    Ok(())
}

#[poise::command(
    context_menu_command = "Translate to English",
    category = "Localisation"
)]
pub async fn translate_to_en(
    ctx: Context<'_>,
    #[description = "Message to translate"] msg: Message,
) -> Result<(), Error> {
    if bee_apartheid(&ctx).await? {
        return Ok(());
    }

    if msg.content.is_empty() {
        empty_warning(&ctx).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let (source_lang, translated_text) = translate_text("en", &msg.content).await?;

    if !check_translated_length(&ctx, translated_text.len()).await? {
        return Ok(());
    }

    let color = ctx.data().colors.translate().await;
    // Send the translated message
    ctx.send_embed(false, |e| {
        e.title("Translated Message")
            .description(format!(
                "{} -> English",
                langcode_to_lang(source_lang).unwrap_or(source_lang)
            ))
            .field("Original Message", &msg.content, false)
            .field("Translated Message", &translated_text, false)
            .color(color)
    })
    .await?;

    Ok(())
}


/// Translates the given text to the given language. The language can be a
/// short name (en) or a long one (English)
///
/// # Output
///
/// If no error occurs it returns a tuple with the source language long name as
/// the first value and the translated string as the second.
async fn translate_text(
    target_lang: &str,
    text_to_translate: &str,
) -> Result<(&'static str, String), Error> {
    let text_target: String = text_to_translate.into();

    let target_lang = get_lang_code(&target_lang)
        .ok_or(
            command_raw_error!("Language not found. Please use the language code or the language name")
        )?.0.to_string();

    if target_lang == "bee" {
        return Ok(("Human", bee_utils::beelate(&text_target)));
    }

    // Turn the provided info into a URI
    let uri = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        target_lang, urlencoding::encode(&text_target).into_owned(),
    );

    // Make the request
    let resp = reqwest::get(&uri).await
        .map_err(
            |why| command_raw_error!("Failed to get translation: {}", why)
        )?;

    // Get the response body and parse it
    let body: Value = serde_json::from_str(
        &resp.text().await
            .map_err(|why| command_raw_error!("Failed to get translation: {}", why))?
    ).map_err(|why| command_raw_error!("Failed to parse response: {}", why))?;

    let sentences = body[0].as_array()
        .ok_or(command_raw_error!("Failed to parse response"))?;


    // Loop over all sentences and turn them into a string
    // NOTE: `concat` may be faster
    let translated_text: String = sentences.iter()
        // Gets the strings inside
        .map(|sentence| sentence[0].as_str())
        // Turns the iterator of Results into an iterator of the Ok values,
        // ignoring Errs
        .flatten()
        // Turns the iterator of strings into a iterator of the individual
        // characters
        .flat_map(|sentence| sentence.chars())
        // Join the characters into a single big String
        .collect();

    // NOTE: the implementation above ignores sentences that aren't strings
    // instead of failing. Please use this one if failing is the required
    // behaviour:
    // Another option is to take a look at:
    // https://users.rust-lang.org/t/iterator-of-results-to-result-of-iterator/38491
    /*
    let mut translated_text = String::new();
    for sentence in sentences {
        translated_text.push_str(
            sentence[0].as_str()
                .map_err(command_raw_error!("Failed to parse response"))?
        );
    }
    */

    // Get the source language
    let source_lang = body[2].as_str()
        .ok_or(
            command_raw_error!("Something went wrong while translating your message")
        )?;

    // If the retrieved source language is not in the list, err
    let (_, source_lang) = get_lang_code(source_lang)
        .ok_or(
            command_raw_error!("Google translate returned an unknown language")
        )?;

    Ok((source_lang, translated_text))
}

/// Tells the user to split the text in different messages if `text_len` is
/// greater than 1024 (Discord's embed field's limit of characters).
///
/// # Output
///
/// If no errors occur, it will return `false` if `text_len > 1024` and the
/// user received a notification or `true` if it's below the limit.
///
/// # Errors
///
/// If `ctx.send_simple(...)` returns an error, it's propagated.
///
/// # Notes
///
/// It may be good to check either the limit is measured in characters
/// (`.chars().count()`) or in bytes (`.len()`) and update this documentation.
async fn check_translated_length(ctx: &Context<'_>, text_len: usize) -> Result<bool, Error> {
    if text_len > 1024 {
        ctx.send_simple(
            true,
            "You entered a text that is too long",
            Some("Please try splitting the original text up into more parts and try again"),
            ctx.data().colors.input_error().await,
        ).await?;
    }
    Ok(text_len <= 1024)
}

/// Finds a short name associated with the given string from [LANGUAGE_CODES].
///
/// # Examples
///
/// ```
/// assert_eq!(get_lang_code("EngLIsH"), "en")
/// assert_eq!(get_lang_code("EN"), "en")
/// assert_eq!(get_lang_code("en"), "en")
/// ```
///
/// # Output
///
/// As long as [s] matches (case-insensitive) a short name or long name from
/// [LANGUAGE_CODES], it will return a tuple with both names (short first).
fn get_lang_code(s: &str) -> Option<(&'static str, &'static str)> {
    // Differently than what some may think, this code doesn't continue
    // iterating after finding the value. Iterators are lazy.
    LANGUAGE_CODES.iter()
        .filter(
            |code| code.0.to_lowercase() == s.to_lowercase()
            || code.1.to_lowercase() == s.to_lowercase()
        )
        .next()
        .copied()
}

/// Sends a message to the user if he's found in `ctx.data().beeified_users`
/// or if the channel is in `ctx.data().beezone_channels`.
///
/// # Output
///
/// True if bee. False is fine.
///
/// # Error
///
/// Propagates `ctx.send_simple(...)`
async fn bee_apartheid(ctx: &Context<'_>) -> Result<bool, Error> {
    let beeified_users = ctx.data().beeified_users.read().await;
    let beezone_channels = ctx.data().beezone_channels.read().await;

    if beeified_users.contains_key(&ctx.author().id)
        || beezone_channels.contains_key(&ctx.channel_id())
    {
        ctx.send_simple(
            false,
            "You are a bee!",
            Some("Bees can't translate, bees can only... bee."),
            ctx.data().colors.bee_translate_block().await,
        ).await?;

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Warns the user that there's no text implying that he tried to translate
/// an empty text.
///
/// # Error
///
/// propagates `ctx.send_simple(...)`
async fn empty_warning(ctx: &Context<'_>) -> Result<(), Error> {
    ctx.send_simple(
        true,
        "There is no text",
        Some("You can't translate nothing into another language"),
        ctx.data().colors.input_error().await,
    ).await?;
    Ok(())
}


#[test]
fn get_lang_code_test() {
    let en = Some(("en", "English"));
    assert_eq!(get_lang_code("EngLIsH"), en);
    assert_eq!(get_lang_code("EN"), en);
    assert_eq!(get_lang_code("en"), en);
}

#[test]
fn langcode_to_lang_test() {
    assert_eq!(langcode_to_lang("amongus"), None);
    assert_eq!(langcode_to_lang("zu"), Some("Zulu"));
}

#[tokio::test]
async fn translate_text_test() {
    // use `cargo test -- --nocapture` to preserve output
    println!("{:?} in en is: {:?}", "sabo muito", translate_text("en", "sabo muito").await);
    println!("Please be sure that this is correct");
}

