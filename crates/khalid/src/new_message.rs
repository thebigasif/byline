//! What a person reads when `khalid new` cannot write a post (RULES.md BL-CLI-3).

use std::io;

use khalid_core::UserMessage;

/// The message when the title has no letter or digit to make a slug from.
pub(crate) fn no_slug(title: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid cannot make an address from the title \"{title}\"."),
        "A post's address is made from the letters and digits in its title.",
        "Give a title with at least one letter or digit.",
    )
}

/// The message when the post's file cannot be written, such as when it is
/// already there, because a post is never replaced.
pub(crate) fn write(relative: &str, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not write {relative}: {error}."),
        "Khalid never replaces a post, and the post exists only once its file is written.",
        "Choose a different title, or check that you may write to the site folder.",
    )
}
