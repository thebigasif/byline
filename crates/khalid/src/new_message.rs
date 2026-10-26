//! What a person reads when `khalid new` cannot write a post (RULES.md BL-CLI-3).

use std::io;

use khalid_core::{Locale, UserMessage};

/// The message when `--language` names a language the site does not list.
pub(crate) fn language(locale: &Locale, languages: &[Locale]) -> UserMessage {
    let listed: Vec<&str> = languages.iter().map(Locale::as_str).collect();
    UserMessage::new(
        format!("The site does not publish in {locale}."),
        "Each post goes in the folder of a language listed in site.toml.",
        format!(
            "Use one of {} with --language, or add {locale} to languages in site.toml.",
            listed.join(", ")
        ),
    )
}

/// The message when the title has no letter or digit to make a slug from.
pub(crate) fn no_slug(title: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid cannot make an address from the title \"{title}\"."),
        "A post's address is made from the letters and digits in its title.",
        "Give a title with at least one letter or digit.",
    )
}

/// The message when the post's file is already there.
pub(crate) fn exists(relative: &str) -> UserMessage {
    UserMessage::new(
        format!("{relative} already exists."),
        "Khalid never overwrites a post, so nothing has changed.",
        "Choose a different title, or open the existing file to edit it.",
    )
}

/// The message when the post's file cannot be written.
pub(crate) fn write(relative: &str, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not write {relative}: {error}."),
        "The post exists only once its file is written.",
        "Check that you may write to the site folder, then run the command again.",
    )
}
