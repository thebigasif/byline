//! What a person reads when `khalid new` cannot write a post (RULES.md BL-CLI-3).

use std::io;

use khalid_core::{Locale, Slug, UserMessage};

use crate::new_author::Byline;
use crate::new_template::Template;

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

/// The message when `--author` has no letter or digit to make a slug from.
pub(crate) fn no_author_slug(name: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid cannot make an author address from the name \"{name}\"."),
        "Each author's page address is made from the letters and digits in their name.",
        "Give --author a name with at least one letter or digit.",
    )
}

/// The lines after "Wrote ...": the author profile `new` made, or, when
/// it named no author, how to add one, since a post needs one to publish.
/// Empty when the post names an author who already has a profile.
pub(crate) fn byline(byline: &Byline<'_>, profile: Option<&str>) -> String {
    match (byline, profile) {
        (Byline::Named { .. }, Some(profile)) => format!(
            "Made the author's profile in {profile}. Add a line about them below its header.\n"
        ),
        (Byline::Named { .. }, None) => String::new(),
        (Byline::Open { profiles }, _) if profiles.is_empty() => {
            "The post has no author yet, and it needs one to publish.\n\
             Next time, run `new` with --author \"Your name\" and Khalid adds you and your profile.\n"
                .to_owned()
        }
        (Byline::Open { profiles }, _) => {
            let listed: Vec<&str> = profiles.iter().map(Slug::as_str).collect();
            format!(
                "The post has no author yet, and it needs one to publish.\n\
                 Add `author:` to its header with one of {}.\n",
                listed.join(", ")
            )
        }
    }
}

/// The message when `--template` asks for a kind of post the title does
/// not read as.
pub(crate) fn not_kind(title: &str, template: Template) -> UserMessage {
    let name = template.name();
    UserMessage::new(
        format!("The title \"{title}\" does not read as a {name} post."),
        format!(
            "The linter checks the shape of a {name} post only when its title says what it is."
        ),
        format!(
            "Write a title like \"{}\", or leave out --template.",
            template.example()
        ),
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
