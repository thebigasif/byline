//! The Admin API's site routes on a running `khalid serve`, writing the
//! site's own files (KH-API-2, audit brief G24).

mod common;

use std::path::PathBuf;

use common::{demo_copy, new_key, start};
use serde_json::json;

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

#[test]
fn a_page_made_edited_and_deleted_through_the_admin_api_is_shown_then_gone() {
    let site = demo_copy();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let new = json!({"pages": [{"slug": "contact", "title": "Contact us", "markdown": "Write to us at the address below."}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/pages/", Some(&new));
    assert_eq!(status, 201, "{made}");
    assert!(server.shows("/blog/contact/", "Contact us"));
    let page = "/api/admin/v1/pages/en.contact/";
    let seen = made["pages"][0]["updated_at"].as_str().unwrap();
    let edit = json!({"pages": [{"title": "Get in touch", "updated_at": seen}]});
    let (status, saved) = server.admin("PUT", page, Some(&edit));
    assert_eq!(status, 200, "{saved}");
    assert!(server.shows("/blog/contact/", "Get in touch"));
    let (status, body) = server.admin("PUT", page, Some(&edit));
    assert_eq!(status, 409, "{body}");

    let seen = saved["pages"][0]["updated_at"].as_str().unwrap();
    let (status, body) = server.admin("DELETE", &format!("{page}?updated_at={seen}"), None);
    assert_eq!(status, 204, "{body}");
    assert!(server.answers("/blog/contact/", 404));
}

#[test]
fn the_demo_snippet_is_edited_in_its_file_and_a_new_one_is_made_and_deleted() {
    let site = demo_copy();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let disclosure = "/api/admin/v1/snippets/en.affiliate-disclosure/";
    let (status, read) = server.admin("GET", disclosure, None);
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["snippets"][0]["title"], "Affiliate disclosure",
        "{read}"
    );
    let seen = read["snippets"][0]["updated_at"].as_str().unwrap();
    let edit = json!({"snippets": [{"markdown": "Some links earn us a fee.", "updated_at": seen}]});
    let (status, body) = server.admin("PUT", disclosure, Some(&edit));
    assert_eq!(status, 200, "{body}");
    let mut file = PathBuf::from(site.path());
    file.push("content/en/snippets/affiliate-disclosure.md");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with("---\ntitle: Affiliate disclosure\nslug: affiliate-disclosure\n---\n")
    );
    assert!(text.ends_with("\nSome links earn us a fee.\n"), "{text}");

    let new = json!({"snippets": [{"language": "de", "slug": "werbung", "title": "Werbung", "markdown": "Bezahlte Links."}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/snippets/", Some(&new));
    assert_eq!(status, 201, "{made}");
    let (_, listed) = server.admin("GET", "/api/admin/v1/snippets/", None);
    assert_eq!(listed["snippets"][1]["id"], "de.werbung", "{listed}");
    let seen = made["snippets"][0]["updated_at"].as_str().unwrap();
    let werbung = format!("/api/admin/v1/snippets/de.werbung/?updated_at={seen}");
    assert_eq!(server.admin("DELETE", &werbung, None).0, 204);
    let mut file = PathBuf::from(site.path());
    file.push("content/de/snippets/werbung.md");
    assert!(!file.exists(), "the German snippet's file is gone");
}
