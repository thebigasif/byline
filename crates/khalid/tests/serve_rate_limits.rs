//! The API's rate limits on a running `khalid serve` (RULES.md BL-SEC-5,
//! audit brief G33).

mod common;

use common::{demo_copy, new_key, start};
use khalid_api::api_limits::ApiLimits;

#[test]
fn an_address_that_sends_wrong_keys_is_paused_on_both_apis() {
    let site = demo_copy();
    let content = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let writer = new_key(site.path(), &["--name", "Writer", "--scope", "posts:write"]);
    let server = start(site.path(), &writer);
    let posts = format!("/api/content/v1/posts/?key={content}&limit=1");
    assert!(server.answers(&posts, 200));
    assert_eq!(server.admin("GET", "/api/admin/v1/posts/", None).0, 200);

    let last = if content.ends_with('0') { "1" } else { "0" };
    let wrong = format!("{}{last}", content.get(..content.len() - 1).unwrap());
    let guess = format!("/api/content/v1/posts/?key={wrong}&limit=1");
    for _ in 0..ApiLimits::REFUSED_PER_MINUTE {
        assert_eq!(server.raw("GET", &guess, &[], None).0, 401);
    }

    let (status, body) = server.raw("GET", &posts, &[], None);
    assert_eq!(status, 429, "{body}");
    assert!(body.contains("too-many-requests"), "{body}");
    let (status, body) = server.admin("GET", "/api/admin/v1/posts/", None);
    assert_eq!(status, 429, "the pause covers the Admin API too: {body}");
}
