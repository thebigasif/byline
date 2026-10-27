//! The Admin API's site routes on a running `khalid serve`, writing the
//! site's own files (KH-API-2, audit brief G24).

mod common;

use std::path::PathBuf;

use common::{demo_copy, new_key, start};

/// The scopes every test here sends: whatever an editor may do.
const EDITOR: [&str; 6] = [
    "--name",
    "Editor",
    "--scope",
    "posts:write",
    "--scope",
    "posts:publish",
];

#[test]
fn a_live_post_deleted_through_the_admin_api_is_gone_with_its_folder() {
    let site = demo_copy();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let post = "/api/admin/v1/posts/en.fixing-slow-pages/";
    let (status, read) = server.admin("GET", post, None);
    assert_eq!(status, 200, "{read}");
    assert!(server.answers("/blog/fixing-slow-pages/", 200));

    let stale = format!("{post}?updated_at=2020-01-01T00:00:00Z");
    let (status, body) = server.admin("DELETE", &stale, None);
    assert_eq!(status, 409, "{body}");
    let seen = read["posts"][0]["updated_at"].as_str().unwrap();
    let (status, body) = server.admin("DELETE", &format!("{post}?updated_at={seen}"), None);
    assert_eq!(status, 204, "{body}");

    let mut folder = PathBuf::from(site.path());
    folder.push("content/en/posts/2026-10-05-fixing-slow-pages");
    assert!(!folder.exists(), "the post's folder and image are gone");
    assert_eq!(server.admin("GET", post, None).0, 404);
    assert!(server.answers("/blog/fixing-slow-pages/", 404));
}
