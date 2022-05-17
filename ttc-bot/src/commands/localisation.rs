use crate::{command_error, utils::autocomplete_functions::language_autocomplete, Context, Error};
use poise::serenity_prelude::{Color, Message};
use serde_json::Value;

pub const LANGUAGE_CODES: [(&str, &str); 104] = [
    ("af", "Afrikaans"),
    ("sq", "Albanian"),
    ("am", "Amharic"),
    ("ar", "Arabic"),
    ("hy", "Armenian"),
    ("az", "Azerbaijani"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bs", "Bosnian"),
    ("bg", "Bulgarian"),
    ("ca", "Catalan"),
    ("ceb", "Cebuano"),
    ("ny", "Chichewa"),
    ("zh-CN", "Chinese (Simplified)"),
    ("zh-TW", "Chinese (Traditional)"),
    ("co", "Corsican"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("eo", "Esperanto"),
    ("et", "Estonian"),
    ("tl", "Filipino"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("fy", "Frisian"),
    ("gl", "Galician"),
    ("ka", "Georgian"),
    ("de", "German"),
    ("el", "Greek"),
    ("gu", "Gujarati"),
    ("ht", "Haitian Creole"),
    ("ha", "Hausa"),
    ("haw", "Hawaiian"),
    ("iw", "Hebrew"),
    ("hi", "Hindi"),
    ("hmn", "Hmong"),
    ("hu", "Hungarian"),
    ("is", "Icelandic"),
    ("ig", "Igbo"),
    ("id", "Indonesian"),
    ("ga", "Irish"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("jw", "Javanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("km", "Khmer"),
    ("ko", "Korean"),
    ("ku", "Kurdish (Kurmanji)"),
    ("ky", "Kyrgyz"),
    ("lo", "Lao"),
    ("la", "Latin"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("lb", "Luxembourgish"),
    ("mk", "Macedonian"),
    ("mg", "Malagasy"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mt", "Maltese"),
    ("mi", "Maori"),
    ("mr", "Marathi"),
    ("mn", "Mongolian"),
    ("my", "Myanmar (Burmese)"),
    ("ne", "Nepali"),
    ("no", "Norwegian"),
    ("ps", "Pashto"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ma", "Punjabi"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sm", "Samoan"),
    ("gd", "Scots Gaelic"),
    ("sr", "Serbian"),
    ("st", "Sesotho"),
    ("sn", "Shona"),
    ("sd", "Sindhi"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("so", "Somali"),
    ("es", "Spanish"),
    ("su", "Sundanese"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tg", "Tajik"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("uz", "Uzbek"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
    ("xh", "Xhosa"),
    ("yi", "Yiddish"),
    ("yo", "Yoruba"),
    ("zu", "Zulu"),
];

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
    #[description = "The text to translate"] text_to_translate: String,
) -> Result<(), Error> {
    // Get the language code and the text to translate

    ctx.defer().await?;

    let (source_lang, translated_text) = translate_text(lang.clone(), &text_to_translate).await?;

    // Send the translated message
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Translated Message")
                .description(format!("{} -> {}", source_lang, lang))
                .field("Original Message", text_to_translate, false)
                .field("Translated Message", translated_text, false)
                .color(Color::FOOYOO)
        })
        .ephemeral(true)
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
    // Get the language code and the text to translate

    ctx.defer().await?;

    let (source_lang, translated_text) = translate_text("en".to_string(), &msg.content).await?;

    // Send the translated message
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Translated Message")
                .description(format!("{} -> English", source_lang))
                .field("Original Message", msg.content, false)
                .field("Translated Message", translated_text, false)
                .color(Color::FOOYOO)
        })
    })
    .await?;

    Ok(())
}

// Function to translate the text
async fn translate_text(
    mut lang: String,
    text_to_translate: &str,
) -> Result<(String, String), Error> {
    let mut language_found = false;

    // Check if the language code is valid
    for lang_code in LANGUAGE_CODES {
        if lang_code.0 == lang {
            language_found = true;
            break;
        } else if lang_code.1.to_lowercase() == lang.to_lowercase() {
            language_found = true;
            lang = lang_code.0.to_string();
            break;
        }
    }

    // If the language code is invalid, return an error
    if !language_found {
        return command_error!(
            "Language not found. Please use the language code or the language name"
        );
    }
    // Turn the provided info into a URI
    let uri = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        lang, text_to_translate,
    );

    // Make the request
    let resp = match reqwest::get(&uri).await {
        Ok(resp) => resp,
        Err(why) => {
            return command_error!("Failed to get translation: {}", why);
        }
    };

    // Get the response body and parse it
    let body: Value = match resp.text().await {
        Ok(body) => match serde_json::from_str(&body) {
            Ok(body) => body,
            Err(why) => {
                return command_error!("Failed to parse response: {}", why);
            }
        },
        Err(why) => {
            return command_error!("Failed to get translation: {}", why);
        }
    };

    let mut translated_text = String::new();

    // Loop over all sentences and turn them into a string
    for sentence in match body[0].as_array() {
        Some(sentence) => sentence,
        None => {
            return command_error!("Failed to parse response");
        }
    } {
        translated_text.push_str(match sentence[0].as_str() {
            Some(sentence) => sentence,
            None => {
                return command_error!("Failed to parse response");
            }
        });
    }

    // Get the source language
    let source_lang = match body[2].as_str() {
        Some(lang) => lang,
        None => {
            return command_error!("Something went wrong while translating your message");
        }
    };

    Ok((source_lang.to_string(), translated_text))
}
