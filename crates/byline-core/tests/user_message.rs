//! The shape of a `UserMessage` on screen and in JSON.

use byline_core::UserMessage;

fn no_author() -> UserMessage {
    UserMessage::new(
        "This post has no author.",
        "Readers and Google trust posts that say who wrote them.",
        format!(
            "Add `author:` to the header of `{}`.",
            "posts/choosing-a-crm.md"
        ),
    )
}

#[test]
fn keeps_the_three_parts_apart() {
    let message = no_author();
    assert_eq!(message.what(), "This post has no author.");
    assert_eq!(
        message.why(),
        "Readers and Google trust posts that say who wrote them."
    );
    assert_eq!(
        message.fix(),
        "Add `author:` to the header of `posts/choosing-a-crm.md`."
    );
}

#[test]
fn displays_what_why_and_fix_on_three_lines() {
    assert_eq!(
        no_author().to_string(),
        "This post has no author.\n\
         Readers and Google trust posts that say who wrote them.\n\
         Add `author:` to the header of `posts/choosing-a-crm.md`."
    );
}

#[test]
fn serialises_to_the_fields_the_api_uses() {
    let json = serde_json::to_value(no_author()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "what": "This post has no author.",
            "why": "Readers and Google trust posts that say who wrote them.",
            "fix": "Add `author:` to the header of `posts/choosing-a-crm.md`.",
        })
    );
}
