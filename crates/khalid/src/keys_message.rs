//! What a person reads when a key command cannot do its work (RULES.md
//! BL-ERR-4).

use std::path::Path;

use khalid_api::keys::KeyId;
use khalid_core::UserMessage;

pub(crate) fn no_name() -> UserMessage {
    UserMessage::new(
        "The key needs a name.",
        "The name tells you later which tool or person holds the key.",
        "Add --name with who uses it, such as --name SerpDad.",
    )
}

pub(crate) fn integration_needs_scopes() -> UserMessage {
    UserMessage::new(
        "An integration key needs at least one scope, and no role.",
        "A scope says what the key may do, such as posts:write to create posts.",
        "Add --scope posts:write, or another scope, and leave out --role.",
    )
}

pub(crate) fn staff_needs_a_role() -> UserMessage {
    UserMessage::new(
        "A staff token needs a role, and no scopes.",
        "The role, such as editor, decides what the staff member may do.",
        "Add --role editor, or owner, admin, writer or contributor, and leave out --scope.",
    )
}

pub(crate) fn content_takes_nothing() -> UserMessage {
    UserMessage::new(
        "A content key takes no scope and no role.",
        "A content key only ever reads published posts, so there is nothing to choose.",
        "Leave out --scope and --role.",
    )
}

pub(crate) fn unknown(id: &KeyId) -> UserMessage {
    UserMessage::new(
        format!("No working key has the identity {}.", id.as_str()),
        "It may be revoked already, or the identity may be mistyped.",
        "Check the 16 characters after the first underscore of the key.",
    )
}

pub(crate) fn not_saved(site: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "Khalid could not save the change to the keys of {}.",
            site.display()
        ),
        "Nothing was changed, so every key works as it did before.",
        "Check the disk has free space and nothing else holds data/site.db, then run it again.",
    )
}

pub(crate) fn no_randomness() -> UserMessage {
    UserMessage::new(
        "Khalid could not get random numbers from the computer to make a key.",
        "A key made without them could be guessed, so no key was made.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}

pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not prepare the new key for storage.",
        "The cause is inside Khalid or the computer, so no key was made.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}
