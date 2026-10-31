//! The Admin API's site routes on a running `khalid serve`, writing the
//! site's own files (KH-API-2, audit brief G24).

mod common;

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use common::{Server, demo_copy, new_key, start};
use serde_json::json;

/// Whether `path` on `server` answers with `text` in its body within 20
/// seconds. Kept here, not in `common`, since only these tests use it and
/// every helper there must be used by each test that shares it.
fn shows(server: &Server, path: &str, text: &str) -> bool {
    (0..200).any(|_| {
        thread::sleep(Duration::from_millis(100));
        server.raw("GET", path, &[], None).1.contains(text)
    })
}

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
    assert!(shows(&server, "/blog/contact/", "Contact us"));
    let page = "/api/admin/v1/pages/en.contact/";
    let seen = made["pages"][0]["updated_at"].as_str().unwrap();
    let edit = json!({"pages": [{"title": "Get in touch", "updated_at": seen}]});
    let (status, saved) = server.admin("PUT", page, Some(&edit));
    assert_eq!(status, 200, "{saved}");
    assert!(shows(&server, "/blog/contact/", "Get in touch"));
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

#[test]
fn an_author_edited_through_the_admin_api_shows_on_their_page_and_a_new_one_goes() {
    let site = demo_copy();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let asif = "/api/admin/v1/authors/en.asif/";
    let (status, read) = server.admin("GET", asif, None);
    assert_eq!(status, 200, "{read}");
    assert_eq!(
        read["authors"][0]["same_as"][0], "https://serpdad.com/",
        "{read}"
    );
    let seen = read["authors"][0]["updated_at"].as_str().unwrap();
    let edit = json!({"authors": [{"role": "Editor of Field Notes", "updated_at": seen}]});
    let (status, body) = server.admin("PUT", asif, Some(&edit));
    assert_eq!(status, 200, "{body}");
    assert!(shows(
        &server,
        "/blog/authors/asif/",
        "Editor of Field Notes"
    ));

    let new = json!({"authors": [{"slug": "sam-lee", "name": "Sam Lee", "same_as": ["https://example.com/sam/"], "bio": "Sam writes about links."}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/authors/", Some(&new));
    assert_eq!(status, 201, "{made}");
    let mut file = PathBuf::from(site.path());
    file.push("content/en/authors/sam-lee.md");
    let text = std::fs::read_to_string(&file).unwrap();
    let front = "name: \"Sam Lee\"\nslug: sam-lee\nsame_as: [\"https://example.com/sam/\"]\n";
    assert!(text.contains(front), "{text}");
    let seen = made["authors"][0]["updated_at"].as_str().unwrap();
    let sam = format!("/api/admin/v1/authors/en.sam-lee/?updated_at={seen}");
    assert_eq!(server.admin("DELETE", &sam, None).0, 204);
    assert!(!file.exists(), "the author's file is gone");
}

#[test]
fn a_tag_renamed_through_the_admin_api_moves_its_posts_and_then_goes() {
    let site = demo_copy();
    let post = |name: &str| {
        let mut path = PathBuf::from(site.path());
        path.push(format!("content/en/posts/{name}.md"));
        path
    };
    let posts = [
        post("2026-09-21-how-to-write-a-title-tag"),
        post("2026-09-28-what-a-sitemap-does"),
    ];
    for path in &posts {
        let text = std::fs::read_to_string(path).unwrap();
        let tagged = text.replacen("\nslug: ", "\ntags: [\"crawling\"]\nslug: ", 1);
        std::fs::write(path, tagged).unwrap();
    }
    let mut draft = PathBuf::from(site.path());
    draft.push("drafts/en");
    std::fs::create_dir_all(&draft).unwrap();
    draft.push("crawl-budget.md");
    let text =
        "---\ntitle: Crawl budget\nslug: crawl-budget\ntags: [crawling, sitemaps]\n---\n\nSoon.\n";
    std::fs::write(&draft, text).unwrap();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let (status, listed) = server.admin("GET", "/api/admin/v1/tags/", None);
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["tags"][0]["id"], "en.crawling", "{listed}");
    assert_eq!(
        listed["tags"][0]["posts"], 3,
        "two posts and a draft: {listed}"
    );
    assert!(server.answers("/blog/tag/crawling/", 200));

    let tag = "/api/admin/v1/tags/en.crawling/";
    let seen = listed["tags"][0]["updated_at"].as_str().unwrap();
    let rename = json!({"tags": [{"name": "Crawling and indexing", "updated_at": seen}]});
    let (status, renamed) = server.admin("PUT", tag, Some(&rename));
    assert_eq!(status, 200, "{renamed}");
    for path in &posts {
        let text = std::fs::read_to_string(path).unwrap();
        assert!(
            text.contains("\ntags: [\"Crawling and indexing\"]\n"),
            "{text}"
        );
    }
    assert!(server.answers("/blog/tag/crawling-and-indexing/", 200));
    assert!(server.answers("/blog/tag/crawling/", 404));

    let seen = renamed["tags"][0]["updated_at"].as_str().unwrap();
    let removal = format!("/api/admin/v1/tags/en.crawling-and-indexing/?updated_at={seen}");
    assert_eq!(server.admin("DELETE", &removal, None).0, 204);
    for path in &posts {
        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains("\ntags:"), "{text}");
    }
    let text = std::fs::read_to_string(&draft).unwrap();
    assert!(
        text.contains("\ntags: [\"sitemaps\"]\n"),
        "the draft keeps its other tag: {text}"
    );
}

#[test]
fn a_category_renamed_through_the_admin_api_shows_on_its_page_and_keeps_its_file() {
    let site = demo_copy();
    let server = start(site.path(), &new_key(site.path(), &EDITOR));
    let (status, listed) = server.admin("GET", "/api/admin/v1/categories/", None);
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["categories"][0]["id"], "on-page-seo", "{listed}");
    assert!(
        listed["categories"][0]["posts"].as_u64().unwrap() > 0,
        "{listed}"
    );

    let on_page = "/api/admin/v1/categories/on-page-seo/";
    let seen = listed["categories"][0]["updated_at"].as_str().unwrap();
    let names = json!({"en": "On-page optimisation", "de": "Onpage-SEO"});
    let edit = json!({"categories": [{"names": names, "updated_at": seen}]});
    let (status, saved) = server.admin("PUT", on_page, Some(&edit));
    assert_eq!(status, 200, "{saved}");
    assert!(shows(
        &server,
        "/blog/category/on-page-seo/",
        "On-page optimisation"
    ));
    let seen = saved["categories"][0]["updated_at"].as_str().unwrap();
    let (status, body) = server.admin("DELETE", &format!("{on_page}?updated_at={seen}"), None);
    assert_eq!(status, 409, "posts are still in it: {body}");

    let new = json!({"categories": [{"slug": "link-building", "names": {"en": "Link building"}}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/categories/", Some(&new));
    assert_eq!(status, 201, "{made}");
    let french = json!({"categories": [{"slug": "liens", "names": {"fr": "Liens"}}]});
    let (status, body) = server.admin("POST", "/api/admin/v1/categories/", Some(&french));
    assert_eq!(status, 422, "the demo is not written in French: {body}");
    let seen = made["categories"][0]["updated_at"].as_str().unwrap();
    let links = format!("/api/admin/v1/categories/link-building/?updated_at={seen}");
    assert_eq!(server.admin("DELETE", &links, None).0, 204);

    let mut file = PathBuf::from(site.path());
    file.push("categories.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with("# The demo site's categories"),
        "comments stay: {text}"
    );
    assert!(text.contains("en = \"On-page optimisation\""), "{text}");
    assert!(!text.contains("link-building"), "{text}");
}

#[test]
fn the_settings_are_changed_in_site_toml_and_applied_after_a_restart() {
    let site = demo_copy();
    let owner = ["--name", "Owner", "--scope", "settings:write"];
    let key = new_key(site.path(), &owner);
    let server = start(site.path(), &key);
    let (status, read) = server.admin("GET", "/api/admin/v1/settings/", None);
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["settings"]["name"], "Field Notes on SEO", "{read}");
    assert_eq!(read["settings"]["restart_needed"], false, "{read}");

    let seen = read["settings"]["updated_at"].as_str().unwrap();
    let change = json!({"settings": {"name": "Field Notes", "updated_at": seen}});
    let (status, saved) = server.admin("PUT", "/api/admin/v1/settings/", Some(&change));
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["settings"]["restart_needed"], true, "{saved}");
    let mut file = PathBuf::from(site.path());
    file.push("site.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with("# The demo site's settings."),
        "comments stay: {text}"
    );
    assert!(text.contains("\nname = \"Field Notes\"\n"), "{text}");
    let (status, body) = server.admin("PUT", "/api/admin/v1/settings/", Some(&change));
    assert_eq!(status, 409, "a replay of the old copy: {body}");

    let seen = saved["settings"]["updated_at"].as_str().unwrap();
    let french = json!({"settings": {"default_language": "fr", "updated_at": seen}});
    let (status, body) = server.admin("PUT", "/api/admin/v1/settings/", Some(&french));
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["rule"], "default_language", "{body}");
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, text, "nothing was written");

    drop(server);
    let restarted = start(site.path(), &key);
    let (_, read) = restarted.admin("GET", "/api/admin/v1/settings/", None);
    assert_eq!(read["settings"]["name"], "Field Notes", "{read}");
    assert_eq!(read["settings"]["restart_needed"], false, "{read}");
}

#[test]
fn a_key_made_through_the_admin_api_opens_the_api_until_it_is_revoked() {
    let site = demo_copy();
    let owner = ["--name", "Owner", "--scope", "admin:all"];
    let server = start(site.path(), &new_key(site.path(), &owner));
    let new =
        json!({"keys": [{"name": "SerpDad", "kind": "integration", "scopes": ["posts:write"]}]});
    let (status, made) = server.admin("POST", "/api/admin/v1/keys/", Some(&new));
    assert_eq!(status, 201, "{made}");
    let key = made["keys"][0]["key"].as_str().unwrap();
    let id = made["keys"][0]["id"].as_str().unwrap();
    let auth = format!("Khalid {key}");
    let headers = [("Authorization", auth.as_str())];
    let (status, _) = server.raw("GET", "/api/admin/v1/posts/", &headers, None);
    assert_eq!(status, 200, "the new key opens the posts list");

    let (status, listed) = server.admin("GET", "/api/admin/v1/keys/", None);
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["keys"][1]["name"], "SerpDad", "{listed}");
    assert!(!listed.to_string().contains(key), "the key is never listed");

    let (status, body) = server.admin("DELETE", &format!("/api/admin/v1/keys/{id}/"), None);
    assert_eq!(status, 204, "{body}");
    let (status, _) = server.raw("GET", "/api/admin/v1/posts/", &headers, None);
    assert_eq!(status, 401, "a revoked key stops at once");
}
