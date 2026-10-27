//! What a person reads when a staff command cannot do its work (RULES.md
//! BL-ERR-4).

use std::path::Path;

use khalid_core::UserMessage;

pub(crate) fn bad_email() -> UserMessage {
    UserMessage::new(
        "That is not an email address.",
        "The owner signs in with this address, and sign-in links go to it.",
        "Give --email with an address such as owner@example.com.",
    )
}

pub(crate) fn no_name() -> UserMessage {
    UserMessage::new(
        "The owner needs a name.",
        "The admin shows the name beside everything the owner changes.",
        "Add --name with the owner's name.",
    )
}

pub(crate) fn owner_taken() -> UserMessage {
    UserMessage::new(
        "This site already has an owner, with another email.",
        "A site has one owner, so nobody can take it over from the command line.",
        "Run this command with the owner's email, or ask the owner to invite you.",
    )
}

pub(crate) fn not_the_owner() -> UserMessage {
    UserMessage::new(
        "This email has an account on the site, but not as the owner.",
        "Only the owner can be signed in from the command line.",
        "Sign in to the admin as usual, or ask the owner to hand the site over.",
    )
}

pub(crate) fn not_saved(site: &Path) -> UserMessage {
    UserMessage::new(
        format!("The owner could not be saved in {}.", site.display()),
        "The site's database holds the accounts, and it could not be written.",
        "Run khalid doctor to check the database, then try again.",
    )
}

pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not make a sign-in link.",
        "The system's random source is not available, and a link must be unguessable.",
        "Try again; if it keeps failing, restart the machine.",
    )
}
