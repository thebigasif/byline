//! The Admin API's posts on a copy of the demo site.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, TimeZone, Utc};
use khalid_api::post_store::{CreatePostError, PostStore, UpdatePostError};
use khalid_api::resources::{AdminPost, PostDraft, PostStatus};
use khalid_core::{Environment, FixedClock};
use serde_json::{Value, json};

use super::*;

fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A copy of the demo site and its posts, on 2026-10-26.
fn demo() -> (tempfile::TempDir, SitePosts) {
    let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&fixture, temp.path());
    let config = Config::load(temp.path(), &Environment::empty()).unwrap();

    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    let posts = SitePosts::new(temp.path(), config, clock);
    (temp, posts)
}

fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// A draft that passes every blocking rule once dated.
fn good_draft() -> PostDraft {
    let markdown = "Anchor text is the visible words of a link.\n\n\
        Read [why internal links matter](/blog/why-internal-links-matter/) first.";
    PostDraft::new("Choosing anchor text", None, markdown)
        .with_author(Some(Slug::try_new("asif").unwrap()))
}

#[test]
fn a_new_post_is_a_draft_file_made_once_per_key() {
    let (site, posts) = demo();
    let made = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k1"))
        .unwrap();
    assert_eq!(made.id(), "en.choosing-anchor-text");
    assert_eq!(made.status(), PostStatus::Draft);
    assert_eq!(made.author().map(Slug::as_str), Some("asif"));
    assert_eq!(made.updated_by(), Some("1111222233334444"));
    let file = within(site.path(), "drafts/en/choosing-anchor-text.md");
    let text = fs::read_to_string(&file).unwrap();
    let front =
        "---\ntitle: \"Choosing anchor text\"\nslug: choosing-anchor-text\nauthor: asif\n---\n";
    assert!(text.starts_with(front), "{text}");

    let again = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k1"))
        .unwrap();
    assert_eq!(again.id(), made.id());
    let taken = posts
        .0
        .create_now(&good_draft(), "1111222233334444", Some("k2"));
    assert!(
        matches!(taken, Err(CreatePostError::SlugTaken(_))),
        "{taken:?}"
    );
    let demo_slug = PostDraft::new("Why internal links matter", None, "Words.");
    let taken = posts.0.create_now(&demo_slug, "1111222233334444", None);
    assert!(
        matches!(taken, Err(CreatePostError::SlugTaken(_))),
        "a published slug is taken too"
    );
}

#[test]
fn a_draft_and_a_published_post_read_back_by_identity() {
    let (_site, posts) = demo();
    posts.0.create_now(&good_draft(), "k", None).unwrap();
    let draft = posts.0.get_now("en.choosing-anchor-text").unwrap().unwrap();
    assert_eq!(draft.markdown(), good_draft().markdown());
    assert_eq!(draft.published_at(), None);

    let live = posts
        .0
        .get_now("de.warum-interne-links-wichtig-sind")
        .unwrap()
        .unwrap();
    assert_eq!(live.status(), PostStatus::Published);
    assert_eq!(
        live.published_at(),
        Some(Utc.with_ymd_and_hms(2026, 9, 14, 0, 0, 0).unwrap())
    );
    assert!(
        !live.markdown().is_empty() && !live.markdown().contains("translation:"),
        "{}",
        live.markdown()
    );

    for unknown in [
        "en.no-such-post",
        "fr.why-internal-links-matter",
        "no-language",
        "en.Bad Slug",
    ] {
        assert!(posts.0.get_now(unknown).unwrap().is_none(), "{unknown}");
    }
}

/// An edit as JSON a client sends.
fn edit(fields: Value) -> PostEdit {
    serde_json::from_value(fields).unwrap()
}

#[tokio::test]
async fn a_stale_edit_changes_nothing() {
    let (_site, posts) = demo();
    posts.create(good_draft(), "k", None).await.unwrap();
    let stale = edit(json!({"title": "New", "updated_at": "2026-01-01T00:00:00Z"}));
    let refused = posts.update("en.choosing-anchor-text", stale, "k").await;
    assert!(
        matches!(refused, Err(UpdatePostError::Stale { .. })),
        "{refused:?}"
    );
    let missing = edit(json!({"title": "New", "updated_at": "2026-01-01T00:00:00Z"}));
    let refused = posts.update("en.no-such-post", missing, "k").await;
    assert!(
        matches!(refused, Err(UpdatePostError::NotFound)),
        "{refused:?}"
    );
}

#[tokio::test]
async fn the_gate_keeps_a_weak_post_a_draft() {
    let (site, posts) = demo();
    let weak = PostDraft::new("A weak post", None, "No link and no author.");
    let made = posts.create(weak, "k", None).await.unwrap();
    let publish = edit(json!({"status": "published", "updated_at": made.updated_at()}));
    let refused = posts.update(made.id(), publish, "k").await;
    let Err(UpdatePostError::Refused(report)) = refused else {
        panic!("published a post the gate blocks: {refused:?}");
    };
    let rules: Vec<Value> = report
        .findings()
        .iter()
        .map(|finding| serde_json::to_value(finding).unwrap()["rule"].clone())
        .collect();
    assert!(rules.contains(&json!("no-author")), "{rules:?}");
    assert!(rules.contains(&json!("no-internal-link")), "{rules:?}");
    assert!(within(site.path(), "drafts/en/a-weak-post.md").is_file());
    assert!(!within(site.path(), "content/en/posts/2026-10-26-a-weak-post.md").exists());
}

#[tokio::test]
async fn a_good_draft_moves_into_content_dated_today() {
    let (site, posts) = demo();
    let made = posts.create(good_draft(), "k", None).await.unwrap();
    let publish = edit(json!({"status": "published", "updated_at": made.updated_at()}));
    let live = posts.update(made.id(), publish, "k").await.unwrap();
    assert_eq!(live.status(), PostStatus::Published);
    let midnight = Utc.with_ymd_and_hms(2026, 10, 26, 0, 0, 0).unwrap();
    assert_eq!(live.published_at(), Some(midnight));
    let file = within(
        site.path(),
        "content/en/posts/2026-10-26-choosing-anchor-text.md",
    );
    assert!(
        fs::read_to_string(file)
            .unwrap()
            .contains("\ndate: 2026-10-26\n")
    );
    assert!(!within(site.path(), "drafts/en/choosing-anchor-text.md").exists());
    let read = posts.get("en.choosing-anchor-text").await.unwrap().unwrap();
    assert_eq!(read.status(), PostStatus::Published);
}

#[tokio::test]
async fn editing_a_published_post_keeps_its_other_fields() {
    let (site, posts) = demo();
    let post = posts
        .get("en.why-internal-links-matter")
        .await
        .unwrap()
        .unwrap();
    let title = "Why internal links matter most";
    let change = edit(json!({"title": title, "updated_at": post.updated_at()}));
    posts.update(post.id(), change, "k").await.unwrap();
    let file = within(
        site.path(),
        "content/en/posts/2026-09-14-why-internal-links-matter.md",
    );
    let text = fs::read_to_string(file).unwrap();
    assert!(
        text.contains("title: \"Why internal links matter most\"\n"),
        "{text}"
    );
    assert!(text.contains("translation: internal-links\n"), "{text}");
    assert!(text.contains("\nAn internal link is a link"), "{text}");
}

#[tokio::test]
async fn a_real_update_writes_today_and_its_note_into_the_file() {
    let (site, posts) = demo();
    let id = "en.why-internal-links-matter";
    let post = posts.get(id).await.unwrap().unwrap();
    assert_eq!(post.updated(), NaiveDate::from_ymd_opt(2026, 10, 20));
    assert_eq!(post.updated_note(), Some("Refreshed the examples."));
    let markdown = format!(
        "{}\n\nOne more example: a link from a guide.",
        post.markdown()
    );
    let note = "Added a guide example.";
    let change = edit(json!({
        "markdown": markdown,
        "real_update": {"note": note},
        "updated_at": post.updated_at(),
    }));
    let saved = posts.update(id, change, "k").await.unwrap();
    assert_eq!(saved.updated(), NaiveDate::from_ymd_opt(2026, 10, 26));
    let file = within(
        site.path(),
        "content/en/posts/2026-09-14-why-internal-links-matter.md",
    );
    let text = fs::read_to_string(file).unwrap();
    assert!(text.contains("\nupdated: 2026-10-26\n"), "{text}");
    assert!(
        text.contains("\nupdated_note: \"Added a guide example.\"\n"),
        "{text}"
    );
    let read = posts.get(id).await.unwrap().unwrap();
    assert_eq!(read.updated_note(), Some(note));
    let title = edit(json!({"title": "Internal links", "updated_at": read.updated_at()}));
    let saved = posts.update(id, title, "k").await.unwrap();
    assert_eq!(saved.updated(), NaiveDate::from_ymd_opt(2026, 10, 26));
}

#[tokio::test]
async fn linting_judges_a_post_and_writes_nothing() {
    let (site, posts) = demo();
    let live = posts.lint("en.how-to-write-a-title-tag").await.unwrap();
    assert!(
        !live.unwrap().blocks(),
        "a published demo post passes the gate"
    );
    assert!(posts.lint("en.no-such-post").await.unwrap().is_none());

    let weak = PostDraft::new("A weak post", None, "No link and no author.");
    posts.create(weak, "k", None).await.unwrap();
    let draft = within(site.path(), "drafts/en/a-weak-post.md");
    let before = fs::read_to_string(&draft).unwrap();
    let report = posts.lint("en.a-weak-post").await.unwrap().unwrap();
    assert!(report.blocks());
    assert_eq!(
        fs::read_to_string(&draft).unwrap(),
        before,
        "the draft is not dated"
    );
}

#[tokio::test]
async fn status_draft_takes_a_live_post_back_into_drafts() {
    let (site, posts) = demo();
    let post = posts
        .get("en.why-internal-links-matter")
        .await
        .unwrap()
        .unwrap();
    let down = edit(json!({"status": "draft", "updated_at": post.updated_at()}));
    let draft = posts.update(post.id(), down, "k").await.unwrap();
    assert_eq!(draft.status(), PostStatus::Draft);
    assert_eq!(draft.published_at(), None);
    let live = "content/en/posts/2026-09-14-why-internal-links-matter.md";
    assert!(!within(site.path(), live).exists());
    let text = fs::read_to_string(within(
        site.path(),
        "drafts/en/why-internal-links-matter.md",
    ));
    assert!(text.unwrap().contains("\nAn internal link is a link"));
    let read = posts.get(post.id()).await.unwrap().unwrap();
    assert_eq!(read.status(), PostStatus::Draft);
}

/// The list `filter` and `order` ask for, every row on one page.
fn query(filter: Option<&str>, order: &str) -> ListQuery {
    let mut raw = khalid_api::query::RawListQuery::default();
    raw.filter = filter.map(str::to_owned);
    raw.order = Some(order.to_owned());
    raw.limit = Some("all".to_owned());
    ListQuery::try_from(&raw).unwrap()
}

#[tokio::test]
async fn the_list_holds_drafts_and_live_posts_with_their_lint_counts() {
    let (_site, posts) = demo();
    let weak = PostDraft::new("A weak post", None, "No link and no author.");
    posts.create(weak, "k", None).await.unwrap();
    posts.create(good_draft(), "k", None).await.unwrap();
    let all = posts.list(&query(None, "id")).await.unwrap();
    let json = serde_json::to_value(all.items()).unwrap();
    let rows = json.as_array().unwrap();
    let row = |id: &str| rows.iter().find(|row| row["id"] == id).unwrap().clone();
    let weak = row("en.a-weak-post");
    assert_eq!(weak["status"], "draft", "{weak}");
    assert_eq!(weak["index_status"], "not_live");
    assert_eq!(weak["gate"], "refuses", "{weak}");
    assert!(weak["lint"]["blocks"].as_u64().unwrap() >= 2, "{weak}");
    let good = row("en.choosing-anchor-text");
    assert_eq!(good["lint"]["blocks"], 0, "a good draft would pass: {good}");
    let live = row("en.why-internal-links-matter");
    assert_eq!(live["status"], "published", "{live}");
    assert_eq!(live["lint"]["blocks"], 0, "{live}");
    assert_eq!(live["index_status"], "indexable", "{live}");
    let live_count = rows
        .iter()
        .filter(|row| row["status"] == "published")
        .count();
    assert_eq!(all.total(), u64::try_from(live_count + 2).unwrap());

    let blocked = posts
        .list(&query(Some("gate:refuses"), "id"))
        .await
        .unwrap();
    let ids: Vec<&str> = blocked.items().iter().map(PostRow::id).collect();
    assert_eq!(ids, ["en.a-weak-post"]);
}

/// The draft made from [`good_draft`], edited with `fields`.
async fn edited_draft(posts: &SitePosts, mut fields: Value) -> Result<AdminPost, UpdatePostError> {
    let made = posts.create(good_draft(), "k", None).await.unwrap();
    fields["updated_at"] = json!(made.updated_at());
    posts.update(made.id(), edit(fields), "k").await
}

#[tokio::test]
async fn settings_go_into_the_front_matter_and_read_back() {
    let (site, posts) = demo();
    let settings = json!({
        "date": "2026-11-02",
        "excerpt": "Words that say where a link goes.",
        "featured": true,
        "robots": "noindex",
        "canonical": "https://example.com/anchor-text/",
        "aliases": ["/2019/05/anchor-text/", "/old/"],
    });
    let saved = edited_draft(&posts, json!({"author": "mira", "settings": settings})).await;
    let saved = saved.unwrap();
    let text = fs::read_to_string(within(site.path(), "drafts/en/choosing-anchor-text.md"));
    let text = text.unwrap();
    for line in [
        "author: mira\n",
        "date: 2026-11-02\n",
        "description: \"Words that say where a link goes.\"\n",
        "featured: true\n",
        "robots: \"noindex, follow\"\n",
        "canonical: \"https://example.com/anchor-text/\"\n",
        "aliases: [\"/2019/05/anchor-text/\", \"/old/\"]\n",
    ] {
        assert!(text.contains(line), "{line} in {text}");
    }
    let read = posts.get(saved.id()).await.unwrap().unwrap();
    assert_eq!(read.settings(), saved.settings());
    assert_eq!(
        serde_json::to_value(read.settings()).unwrap()["featured"],
        true
    );
}

#[tokio::test]
async fn co_authors_image_review_and_share_cards_go_into_the_front_matter() {
    let (site, posts) = demo();
    let draft = within(site.path(), "drafts/en/choosing-anchor-text.md");
    for (settings, line) in [
        (json!({"co_authors": ["mira"]}), "co_authors: [mira]\n"),
        (json!({"review_state": "ready"}), "review_state: ready\n"),
        (
            json!({"feature_image": {"src": "/media/a.jpg", "alt": "A link"}}),
            "image: {src: \"/media/a.jpg\", alt: \"A link\"}\n",
        ),
        (
            json!({"share_cards": {"x": {"title": "Anchor text"}}}),
            "share_cards: {x: {title: \"Anchor text\"}}\n",
        ),
    ] {
        let saved = edited_draft(&posts, json!({"settings": settings})).await;
        let saved = saved.unwrap();
        let text = fs::read_to_string(&draft).unwrap();
        assert!(text.contains(line), "{line} in {text}");
        let read = posts.get(saved.id()).await.unwrap().unwrap();
        assert_eq!(read.settings(), saved.settings());
        fs::remove_file(&draft).unwrap();
    }
}

#[tokio::test]
async fn access_password_template_and_head_code_go_into_the_file() {
    let (site, posts) = demo();
    let mut templates = within(site.path(), "theme/templates");
    fs::create_dir_all(&templates).unwrap();
    templates.push("post-wide.html");
    fs::write(&templates, "{% extends \"post.html\" %}").unwrap();
    let code = "<script>\n// a comment\ngo(\"x\");\n</script>";
    let settings = json!({
        "access": "members",
        "password": "generated-for-this-test",
        "template": "wide",
        "head_code": code,
    });
    let made = posts.create(good_draft(), "k", None).await.unwrap();
    let mut change = edit(json!({"settings": settings, "updated_at": made.updated_at()}));
    change.hash_password().unwrap();
    let saved = posts.update(made.id(), change, "k").await.unwrap();
    let file = within(site.path(), "drafts/en/choosing-anchor-text.md");
    let text = fs::read_to_string(&file).unwrap();
    for line in [
        "access: members\n",
        "password_hash: \"$argon2id$",
        "template: wide\n",
        "head_code: \"<script>\\n// a comment\\ngo(\\\"x\\\");\\n</script>\"\n",
    ] {
        assert!(text.contains(line), "{line} in {text}");
    }
    assert!(!text.contains("generated-for-this-test"), "{text}");
    let read = posts.get(saved.id()).await.unwrap().unwrap();
    assert_eq!(read.settings(), saved.settings());
    assert_eq!(
        read.settings().head_code.as_ref().unwrap().to_string(),
        code
    );

    let clear = json!({"access": "public", "password": null, "template": null, "head_code": null});
    let mut change = edit(json!({"settings": clear, "updated_at": read.updated_at()}));
    change.hash_password().unwrap();
    posts.update(read.id(), change, "k").await.unwrap();
    let text = fs::read_to_string(&file).unwrap();
    for field in ["access:", "password_hash:", "template:", "head_code:"] {
        assert!(!text.contains(field), "{field} in {text}");
    }
}

#[tokio::test]
async fn a_template_the_theme_lacks_is_refused() {
    let (_site, posts) = demo();
    let refused = edited_draft(&posts, json!({"settings": {"template": "wide"}})).await;
    assert!(
        matches!(
            refused,
            Err(UpdatePostError::Unsupported {
                setting: "template",
                ..
            })
        ),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_new_slug_moves_a_draft_but_not_a_live_post() {
    let (site, posts) = demo();
    let moved = edited_draft(&posts, json!({"slug": "anchor-text-guide"})).await;
    let moved = moved.unwrap();
    assert_eq!(moved.id(), "en.anchor-text-guide");
    assert!(!within(site.path(), "drafts/en/choosing-anchor-text.md").exists());
    let text = fs::read_to_string(within(site.path(), "drafts/en/anchor-text-guide.md"));
    assert!(text.unwrap().contains("\nslug: anchor-text-guide\n"));

    let taken = edited_draft(&posts, json!({"slug": "anchor-text-guide"})).await;
    assert!(
        matches!(taken, Err(UpdatePostError::SlugTaken(_))),
        "{taken:?}"
    );
    let live = posts
        .get("en.why-internal-links-matter")
        .await
        .unwrap()
        .unwrap();
    let change = edit(json!({"slug": "internal-links", "updated_at": live.updated_at()}));
    let refused = posts.update(live.id(), change, "k").await;
    let Err(UpdatePostError::Unsupported { setting, .. }) = refused else {
        panic!("moved a live post: {refused:?}");
    };
    assert_eq!(setting, "slug");
    let change = edit(json!({"settings": {"date": null}, "updated_at": live.updated_at()}));
    let refused = posts.update(live.id(), change, "k").await;
    assert!(matches!(
        refused,
        Err(UpdatePostError::Unsupported {
            setting: "date",
            ..
        })
    ));
}

#[test]
fn a_trash_key_names_one_folder_inside_trash_and_never_climbs() {
    let (_site, posts) = demo();
    let names = |key: &str| posts.0.names_one_copy(key);
    assert!(names("20261031T093000Z-en.hello"));
    for climbing in ["..", "20261031T093000Z-x/../..", "a/b", "/etc", ""] {
        assert!(!names(climbing), "{climbing}");
    }
}
