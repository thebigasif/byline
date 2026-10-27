//! What a person reads when `khalid mcp` cannot start or stops early
//! (RULES.md BL-ERR-4). These go to stderr: stdout carries only the
//! protocol.

use khalid_core::UserMessage;

/// How to make a key for an agent, the same in every message.
const MAKE_A_KEY: &str = "khalid keys create --scope posts:write --name Claude";

pub(crate) fn no_key() -> UserMessage {
    UserMessage::new(
        "No key was given to khalid mcp in KHALID_KEY.",
        "The agent works on the site with a key, so the server does not start without one.",
        format!("Make one with {MAKE_A_KEY}, and set KHALID_KEY to it in the agent's settings."),
    )
}

pub(crate) fn refused() -> UserMessage {
    UserMessage::new(
        "The key in KHALID_KEY was not accepted.",
        "It may be mistyped or revoked, and the server does not act without a working key.",
        format!("Make a new key with {MAKE_A_KEY}, and set KHALID_KEY to it."),
    )
}

pub(crate) fn content_key() -> UserMessage {
    UserMessage::new(
        "The key in KHALID_KEY is a content key, which can only read.",
        "An agent creates and edits posts through khalid mcp, so it needs a key that can write.",
        format!("Make an integration key with {MAKE_A_KEY}, and use that one."),
    )
}

pub(crate) fn unchecked() -> UserMessage {
    UserMessage::new(
        "The key in KHALID_KEY could not be checked.",
        "The site's database could not be read, so the server cannot tell who is asking.",
        "Run khalid doctor to find the problem, then start the agent again.",
    )
}

pub(crate) fn stopped() -> UserMessage {
    UserMessage::new(
        "khalid mcp stopped before the agent was done.",
        "A message could not be read or answered, or was longer than 8 MB, so the session ended.",
        "Start the agent again. If it keeps happening, update the agent and Khalid.",
    )
}
