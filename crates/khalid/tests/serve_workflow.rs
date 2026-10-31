//! The editorial workflow on a running `khalid serve`: the calendar shows
//! the demo's posts and a dated draft by day, and comments on a post are
//! kept in the database and resolved (KH-EDT-4).

mod common;

use serde_json::{Value, json};

#[test]
fn the_calendar_shows_the_live_posts_and_a_dated_draft_by_day() {
    let site = common::demo_copy();
    let key = common::new_key(site.path(), &["--name", "Desk", "--scope", "posts:write"]);
    let server = common::start(site.path(), &key);
    let draft = json!({"posts": [{"title": "Choosing anchor text", "author": "asif", "markdown": "Text."}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/posts/", Some(&draft));
    assert_eq!(status, 201, "{made}");
    assert!(
        server.answers("/blog/choosing-anchor-text/", 404),
        "a draft is never served"
    );
    let dated = json!({"posts": [{
        "settings": {"date": "2026-09-30", "review_state": "in_review"},
        "updated_at": made["posts"][0]["updated_at"],
    }]});
    let path = "/api/admin/v1/posts/en.choosing-anchor-text/";
    let (status, body) = server.admin("PUT", path, Some(&dated));
    assert_eq!(status, 200, "{body}");

    let month = "/api/admin/v1/calendar/?from=2026-09-01&to=2026-09-30";
    let (status, body) = server.admin("GET", month, None);
    assert_eq!(status, 200, "{body}");
    let days = body["calendar"]["days"].as_array().unwrap();
    let on = |date: &str, id: &str| -> Value {
        let day = days.iter().find(|day| day["date"] == date).unwrap();
        let posts = day["posts"].as_array().unwrap();
        posts.iter().find(|post| post["id"] == id).unwrap().clone()
    };
    let live = on("2026-09-14", "en.why-internal-links-matter");
    assert_eq!(live["status"], "published", "{body}");
    let planned = on("2026-09-30", "en.choosing-anchor-text");
    assert_eq!(planned["status"], "draft", "{body}");
    assert_eq!(planned["review_state"], "in_review", "{body}");
    assert_eq!(body["calendar"]["undated"], json!([]), "{body}");
}

#[test]
fn a_comment_on_a_demo_post_is_kept_through_a_restart_and_resolved() {
    let site = common::demo_copy();
    let key = common::new_key(site.path(), &["--name", "Desk", "--scope", "posts:write"]);
    let comments = "/api/admin/v1/posts/en.what-a-sitemap-does/draft-comments/";
    let first = json!({"draft_comments": [{"text": "Add the 50,000 limit.", "block": 3}]});
    {
        let server = common::start(site.path(), &key);
        let (status, body) = server.admin("POST", comments, Some(&first));
        assert_eq!(status, 201, "{body}");
        let thread = body["draft_comments"][0]["id"].clone();
        let reply = json!({"draft_comments": [{"text": "Added.", "parent": thread}]});
        let (status, body) = server.admin("POST", comments, Some(&reply));
        assert_eq!(status, 201, "{body}");
    }
    let server = common::start(site.path(), &key);
    let (status, body) = server.admin("GET", comments, None);
    assert_eq!(status, 200, "{body}");
    let listed = body["draft_comments"].as_array().unwrap();
    assert_eq!(listed.len(), 2, "{body}");
    assert_eq!(listed[1]["block"], 3, "{body}");
    let thread = format!("{comments}{}/", listed[0]["id"]);
    let resolve = json!({"draft_comments": [{"resolved": true}]});
    let (status, body) = server.admin("PUT", &thread, Some(&resolve));
    assert_eq!(status, 204, "{body}");
    let (_, body) = server.admin("GET", comments, None);
    assert_eq!(body["draft_comments"][0]["resolved"], true, "{body}");
}
