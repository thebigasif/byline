//! `SerpDad`'s draft target on a running `khalid serve`: a finished
//! article, sent the way `SerpDad` sends one to `WordPress`, lands as a
//! linted draft in under 5 seconds, and the answer carries the lint result
//! and where the draft opens in the admin (KH-API-6, brief G56).

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use common::{Server, demo_copy, new_key, start};
use serde_json::{Value, json};

/// The plan's promise for publishing from `SerpDad` (plan section 11).
const DEADLINE: Duration = Duration::from_secs(5);

/// Where `SerpDad` stages drafts, with HTML bodies.
const STAGE: &str = "/api/admin/v1/posts/?source=html";

/// A key that writes drafts and nothing more, as `SerpDad` is told to make.
const SERPDAD: [&str; 4] = ["--name", "SerpDad", "--scope", "posts:write"];

/// Sends `article` once with `key` and the idempotency key `request`, and
/// returns the status, the answer and whether it came within
/// [`DEADLINE`]. The request runs on its own thread and the answer is
/// waited for, since no Clock can see a separate process.
#[cfg(test)]
fn stage(server: &Server, key: &str, request: &str, article: &Value) -> (u16, Value, bool) {
    let authorization = format!("Khalid {key}");
    let headers = [
        ("Authorization", authorization.as_str()),
        ("Idempotency-Key", request),
    ];
    let (sent, answered) = mpsc::channel();
    let (in_time, (status, body)) = thread::scope(|scope| {
        scope.spawn(|| {
            let _ = sent.send(server.raw("POST", STAGE, &headers, Some(article)));
        });
        match answered.recv_timeout(DEADLINE) {
            Ok(answer) => (true, answer),
            Err(_) => (false, answered.recv().unwrap()),
        }
    });
    let answer = serde_json::from_str(&body).unwrap_or_else(|_| panic!("{body}"));
    (status, answer, in_time)
}

/// The findings in `answer`'s report that have `severity`.
#[cfg(test)]
fn findings(answer: &Value, severity: &str) -> usize {
    let all = answer["lint"]["report"]["findings"].as_array().unwrap();
    all.iter().filter(|f| f["severity"] == severity).count()
}

#[test]
fn a_finished_article_lands_as_a_linted_draft_in_under_five_seconds() {
    let site = demo_copy();
    let key = new_key(site.path(), &SERPDAD);
    let server = start(site.path(), &key);
    let article = json!({"posts": [{
        "title": "Choosing anchor text",
        "slug": "choosing-anchor-text",
        "author": "asif",
        "excerpt": "Pick the words a reader clicks with care.",
        "html": "<p>Anchor text is the visible words of a link.</p>\
            <h2>Why it matters</h2>\
            <p>Read <a href=\"/blog/why-internal-links-matter/\">why internal links matter</a> first.</p>",
    }]});

    let (status, made, in_time) = stage(&server, &key, "serpdad-article-381", &article);
    assert_eq!(status, 201, "{made}");
    assert!(in_time, "staging took longer than {DEADLINE:?}");
    let draft = &made["posts"][0];
    assert_eq!(draft["id"], "en.choosing-anchor-text", "{made}");
    assert_eq!(draft["status"], "draft", "{made}");
    let lint = &made["lint"];
    assert_eq!(lint["blocks"], findings(&made, "block"), "{made}");
    assert_eq!(lint["warnings"], findings(&made, "warn"), "{made}");
    assert!(lint["report"]["score"]["value"].is_u64(), "{made}");
    assert_eq!(made["admin_path"], "/admin/posts/en.choosing-anchor-text");

    let mut file = PathBuf::from(site.path());
    file.push("drafts/en/choosing-anchor-text.md");
    let text = fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("description: \"Pick the words a reader clicks with care.\""),
        "{text}"
    );
    assert!(text.contains("[why internal links matter]"), "{text}");
    let (status, read) = server.admin("GET", "/api/admin/v1/posts/en.choosing-anchor-text/", None);
    assert_eq!(status, 200, "{read}");
    let excerpt = &read["posts"][0]["settings"]["excerpt"];
    assert_eq!(
        excerpt, "Pick the words a reader clicks with care.",
        "{read}"
    );
    assert!(
        server.answers("/blog/choosing-anchor-text/", 404),
        "a draft is never served"
    );

    let (status, again, _) = stage(&server, &key, "serpdad-article-381", &article);
    assert_eq!(status, 201, "{again}");
    assert_eq!(
        again["posts"][0]["id"], draft["id"],
        "a retry makes no second post"
    );
    assert_eq!(again["lint"]["blocks"], lint["blocks"], "{again}");
}

#[test]
fn an_article_the_linter_would_hold_is_staged_with_its_blocks_named() {
    let site = demo_copy();
    let key = new_key(site.path(), &SERPDAD);
    let server = start(site.path(), &key);
    let article = json!({"posts": [{"title": "A weak post", "html": "<p>Short.</p>"}]});

    let (status, made, in_time) = stage(&server, &key, "serpdad-article-382", &article);
    assert_eq!(status, 201, "{made}");
    assert!(in_time, "staging took longer than {DEADLINE:?}");
    assert_eq!(made["posts"][0]["status"], "draft", "{made}");
    assert!(
        made["lint"]["blocks"].as_u64().is_some_and(|n| n >= 1),
        "{made}"
    );
    let rules = made["lint"]["report"]["findings"].as_array().unwrap();
    assert!(rules.iter().any(|f| f["rule"] == "no-author"), "{made}");
    assert!(server.answers("/blog/a-weak-post/", 404));
}

#[test]
fn the_connect_page_makes_the_key_lists_its_drafts_and_turns_it_off() {
    let site = demo_copy();
    let owner = new_key(
        site.path(),
        &["--name", "Owner", "--scope", "settings:write"],
    );
    let server = start(site.path(), &owner);
    let (status, off) = server.admin("GET", "/api/admin/v1/serpdad/", None);
    assert_eq!(status, 200, "{off}");
    assert_eq!(off, json!({"connection": null, "drafts": []}));

    let (status, made) = server.admin("POST", "/api/admin/v1/serpdad/", None);
    assert_eq!(status, 201, "{made}");
    let key = made["connection"]["key"].as_str().unwrap();
    let article = json!({"posts": [{"title": "Choosing anchor text", "author": "asif",
        "html": "<p>Anchor text is the visible words of a link.</p>"}]});
    let (status, staged, _) = stage(&server, key, "serpdad-article-383", &article);
    assert_eq!(status, 201, "{staged}");

    let (_, read) = server.admin("GET", "/api/admin/v1/serpdad/", None);
    let draft = &read["drafts"][0];
    assert_eq!(draft["id"], "en.choosing-anchor-text", "{read}");
    assert_eq!(draft["admin_path"], "/admin/posts/en.choosing-anchor-text");
    assert_eq!(draft["lint"]["blocks"], staged["lint"]["blocks"], "{read}");
    assert_eq!(draft["lint"]["warnings"], staged["lint"]["warnings"]);
    assert_eq!(read["connection"].get("key"), None, "the key is shown once");

    let (status, _) = server.admin("DELETE", "/api/admin/v1/serpdad/", None);
    assert_eq!(status, 204);
    let (status, refused, _) = stage(&server, key, "serpdad-article-384", &article);
    assert_eq!(status, 401, "a switched-off key is refused: {refused}");
}
