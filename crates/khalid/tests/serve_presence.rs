//! Presence and the post lock on a running `khalid serve`: the first
//! writer to open a post holds it, a second one reads, and the lock passes
//! on when the first leaves (plan 4.6 and 5.12, KH-EDT-2).

mod common;

use serde_json::{Value, json};

/// The demo post both writers open.
const PRESENCE: &str = "/api/admin/v1/posts/en.why-internal-links-matter/presence/";

/// Checks in on the post with `key` and answers what it saw.
#[cfg(test)]
fn check_in(server: &common::Server, key: &str) -> Value {
    let join = json!({ "claim": "join" });
    let auth = format!("Khalid {key}");
    let headers = [("Authorization", auth.as_str())];
    let (status, body) = server.raw("POST", PRESENCE, &headers, Some(&join));
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[test]
fn the_second_writer_reads_until_the_first_leaves() {
    let site = common::demo_copy();
    let scopes = ["--scope", "posts:write"];
    let maria = common::new_key(site.path(), &[&["--name", "Maria"][..], &scopes].concat());
    let lena = common::new_key(site.path(), &[&["--name", "Lena"][..], &scopes].concat());
    let server = common::start(site.path(), &maria);
    assert_eq!(check_in(&server, &maria)["yours"], true);
    let seen = check_in(&server, &lena);
    assert_eq!(seen["yours"], false, "{seen}");
    let others = &check_in(&server, &maria)["others"];
    assert_eq!(others.as_array().map(Vec::len), Some(1), "{others}");
    let (status, json) = server.admin("DELETE", PRESENCE, None);
    assert_eq!(status, 204, "{json}");
    assert_eq!(check_in(&server, &lena)["yours"], true);
    assert!(
        server.answers("/blog/why-internal-links-matter/", 200),
        "readers never see the lock"
    );
}
