//! The audit log on a running `khalid serve` (KH-EDT-6, brief G52): a post
//! made through the Admin API is logged with the key that made it, and the
//! log outlasts a restart.

mod common;

use serde_json::json;

use common::{demo_copy, new_key, start};

#[test]
fn a_change_through_the_admin_api_is_logged_with_who_made_it_and_kept() {
    let site = demo_copy();
    let key = new_key(site.path(), &["--name", "Owner", "--scope", "admin:all"]);
    let by = key.split('_').nth(1).unwrap().to_owned();
    let server = start(site.path(), &key);
    let new = json!({"posts": [{"title": "Choosing anchor text", "markdown": "Words."}]});
    let (status, body) = server.admin("POST", "/api/admin/v1/posts/", Some(&new));
    assert_eq!(status, 201, "{body}");
    drop(server);

    let server = start(site.path(), &key);
    assert!(
        server.answers("/api/admin/v1/audit/", 401),
        "the log needs a key"
    );
    let (status, log) = server.admin("GET", "/api/admin/v1/audit/", None);
    assert_eq!(status, 200, "{log}");
    let change = &log["changes"][0];
    assert_eq!(change["by"], by.as_str(), "{log}");
    assert_eq!(change["action"], "added", "{log}");
    assert_eq!(change["route"], "/api/admin/v1/posts/", "{log}");
    assert_eq!(log["changes"].as_array().map(Vec::len), Some(1), "{log}");
}
