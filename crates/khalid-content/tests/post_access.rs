//! Front matter reads who may read a post, its template, its password hash
//! and its head code, and refuses values that cannot be right.

use std::path::Path;

use khalid_content::{PostFront, parse_post};
use khalid_core::PostAccess;

const HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$aGFzaGhhc2g";

#[cfg(test)]
fn front(yaml: &str) -> PostFront {
    let text = format!("---\ntitle: A\n{yaml}\n---\nBody.\n");
    parse_post(Path::new("posts/a.md"), &text).unwrap().front
}

#[cfg(test)]
fn message(yaml: &str) -> String {
    let text = format!("---\ntitle: A\n{yaml}\n---\nBody.\n");
    parse_post(Path::new("posts/a.md"), &text)
        .unwrap_err()
        .to_string()
}

#[test]
fn a_post_is_public_with_no_template_password_or_head_code_by_default() {
    let plain = front("date: 2026-10-27");
    assert_eq!(plain.access, PostAccess::Public);
    assert_eq!(plain.template, None);
    assert_eq!(plain.password_hash, None);
    assert_eq!(plain.head_code, None);
}

#[test]
fn reads_access_template_password_hash_and_head_code() {
    let yaml = format!(
        "access: members\ntemplate: wide\npassword_hash: \"{HASH}\"\nhead_code: \"<meta name=verify content=x>\""
    );
    let post = front(&yaml);
    assert_eq!(post.access, PostAccess::Members);
    assert_eq!(post.template.unwrap().file_name(), "post-wide.html");
    assert_eq!(post.password_hash.unwrap().as_str(), HASH);
    let code = post.head_code.unwrap().to_string();
    assert_eq!(code, "<meta name=\"verify\" content=\"x\">");
}

#[test]
fn refuses_what_cannot_be_right_and_says_which_field() {
    let cases = [
        ("access: paid", "access"),
        ("template: ../base", "template"),
        ("password_hash: hunter2", "password_hash"),
        ("head_code: \"<div>hi</div>\"", "head_code"),
    ];
    for (yaml, field) in cases {
        let message = message(yaml);
        assert!(message.contains(field), "{yaml}: {message}");
    }
    let plain = message("password: hunter2");
    assert!(plain.contains("password"), "{plain}");
}
