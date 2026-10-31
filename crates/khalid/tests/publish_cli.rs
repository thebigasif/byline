//! `khalid publish` takes a draft live through the same publish door the
//! Admin API uses: the lint gate first, then the signed webhooks and
//! `IndexNow`, in human and JSON output (KH-OPS-2, KH-LNT-1, KH-API-3,
//! audit brief G28).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The stand-in hosts for the webhook receiver and `IndexNow`, on loopback.
const HOSTS: &str = "hooks.test=127.0.0.1,indexnow.test=127.0.0.1";

/// A draft the gate lets through.
const GOOD: &str = "---\ntitle: \"Choosing anchor text\"\nslug: choosing-anchor-text\n\
    author: asif\n---\nAnchor text is the visible words of a link.\n\n\
    Read [why internal links matter](/blog/why-internal-links-matter/) first.\n";

/// A draft the gate refuses.
const WEAK: &str = "---\ntitle: \"A weak post\"\nslug: a-weak-post\n---\nShort.\n";

/// A copy of the demo site sending `IndexNow` and a webhook for each of
/// `post.published` and `lint.blocked` to `receiver`, at `/indexnow` and
/// `/<event>`. Returns the site and the two secrets, in that order.
#[cfg(test)]
async fn demo_site(receiver: &MockServer) -> (TempDir, [Secret<String>; 2]) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let port = receiver.address().port();
    let settings = at(temp.path(), "site.toml");
    let toml = format!(
        "{}\n[seo.indexnow]\nkey = \"3f9c0b5e2a8d4c71\"\n\
         endpoint = \"http://indexnow.test:{port}/indexnow\"\n",
        fs::read_to_string(&settings).unwrap()
    );
    fs::write(&settings, toml).unwrap();
    fs::create_dir_all(at(temp.path(), "data")).unwrap();
    let db = Database::open(&at(temp.path(), "data/site.db"))
        .await
        .unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    let mut secrets = Vec::new();
    for event in [Event::PostPublished, Event::LintBlocked] {
        let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/{event}")).unwrap();
        secrets.push(table.create(&url, event).await.unwrap().1);
    }
    db.close().await;
    (temp, secrets.try_into().unwrap())
}

/// `rest` inside `dir`.
fn at(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(rest);
    path
}

/// A receiver that takes every request.
#[cfg(test)]
async fn receiver() -> MockServer {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&receiver)
        .await;
    receiver
}

/// Writes `text` as the draft `slug` of the site in `site`.
#[cfg(test)]
fn draft(site: &Path, slug: &str, text: &str) -> PathBuf {
    let folder = at(site, "drafts/en");
    fs::create_dir_all(&folder).unwrap();
    let file = at(&folder, &format!("{slug}.md"));
    fs::write(&file, text).unwrap();
    file
}

/// Runs `khalid publish` with `args` on `site`, reaching the stand-ins.
#[cfg(test)]
fn publish(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "--test-hosts", HOSTS])
        .arg("publish")
        .args(args)
        .output()
        .unwrap()
}

/// What `receiver` got at `route`. The command waits for its deliveries
/// before it ends, so there is nothing to wait for.
#[cfg(test)]
async fn received(receiver: &MockServer, route: &str) -> Vec<Request> {
    let all = receiver.received_requests().await.unwrap();
    all.into_iter().filter(|r| r.url.path() == route).collect()
}

/// The one delivery of `event`, checked against `secret`, as JSON.
#[cfg(test)]
async fn the_delivery(receiver: &MockServer, event: Event, secret: &Secret<String>) -> Value {
    let hooks = received(receiver, &format!("/{event}")).await;
    assert_eq!(hooks.len(), 1, "{event} fires once");
    let signature = hooks[0].headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(secret).unwrap());
    verifier
        .verify(signature, &hooks[0].body, &SystemClock)
        .unwrap();
    serde_json::from_slice(&hooks[0].body).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_draft_published_from_the_command_line_goes_live_and_is_announced() {
    let receiver = receiver().await;
    let (site, [published, _]) = demo_site(&receiver).await;
    let file = draft(site.path(), "choosing-anchor-text", GOOD);

    let output = publish(site.path(), &["choosing-anchor-text", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer["posts"][0]["id"], "en.choosing-anchor-text");
    assert_eq!(answer["posts"][0]["status"], "published", "{answer}");
    assert!(!file.exists(), "the draft file moved into the content");

    let body = the_delivery(&receiver, Event::PostPublished, &published).await;
    assert_eq!(body["post"]["id"], "en.choosing-anchor-text", "{body}");
    let searches = received(&receiver, "/indexnow").await;
    assert_eq!(searches.len(), 1, "IndexNow is told once");
    let search: Value = serde_json::from_slice(&searches[0].body).unwrap();
    assert_eq!(
        search["urlList"],
        json!(["https://www.example.com/blog/choosing-anchor-text/"])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_weak_draft_is_refused_and_fires_lint_blocked() {
    let receiver = receiver().await;
    let (site, [_, blocked]) = demo_site(&receiver).await;
    let file = draft(site.path(), "a-weak-post", WEAK);

    let output = publish(site.path(), &["a-weak-post", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    let what = answer["error"]["what"].as_str().unwrap_or_default();
    assert!(what.starts_with("Khalid did not publish"), "{answer}");
    assert_eq!(answer["lint"]["blocks"], true, "{answer}");
    assert_eq!(fs::read_to_string(&file).unwrap(), WEAK, "the draft stays");

    let body = the_delivery(&receiver, Event::LintBlocked, &blocked).await;
    assert_eq!(body["post"]["id"], "en.a-weak-post", "{body}");
    assert_eq!(body["lint"]["blocks"], true, "{body}");
    assert!(received(&receiver, "/post.published").await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_human_output_names_the_post_or_every_block() {
    let receiver = receiver().await;
    let (site, _) = demo_site(&receiver).await;
    draft(site.path(), "choosing-anchor-text", GOOD);
    draft(site.path(), "a-weak-post", WEAK);

    let output = publish(site.path(), &["choosing-anchor-text"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    assert_eq!(said, "Published en/choosing-anchor-text.\n");

    let output = publish(site.path(), &["a-weak-post"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let findings = String::from_utf8(output.stdout).unwrap();
    assert!(
        findings.contains("en/a-weak-post: blocks publishing"),
        "{findings}"
    );
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(
        message.starts_with("Khalid did not publish en/a-weak-post"),
        "{message}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_undated_post_in_content_is_a_draft_and_goes_live_where_it_is() {
    let receiver = receiver().await;
    let (site, [published, _]) = demo_site(&receiver).await;
    let file = at(site.path(), "content/en/posts/choosing-anchor-text.md");
    fs::write(&file, GOOD).unwrap();

    let output = publish(site.path(), &["choosing-anchor-text"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    assert_eq!(said, "Published en/choosing-anchor-text.\n");
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("\ndate: 20"), "the post is dated: {text}");
    let body = the_delivery(&receiver, Event::PostPublished, &published).await;
    assert_eq!(body["post"]["status"], "published", "{body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_already_live_is_refused_not_published_again() {
    let receiver = receiver().await;
    let (site, _) = demo_site(&receiver).await;
    let file = at(
        site.path(),
        "content/en/posts/2026-09-14-why-internal-links-matter.md",
    );
    let before = fs::read_to_string(&file).unwrap();

    let output = publish(site.path(), &["why-internal-links-matter"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(
        message.starts_with(
            "Khalid did not publish en/why-internal-links-matter, because it is already live."
        ),
        "{message}"
    );
    assert!(message.contains("drafts/en/"), "{message}");
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
    assert!(received(&receiver, "/post.published").await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slug_with_no_post_is_named_with_where_to_look() {
    let receiver = receiver().await;
    let (site, _) = demo_site(&receiver).await;

    let output = publish(site.path(), &["no-such-post", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    let what = answer["error"]["what"].as_str().unwrap_or_default();
    assert_eq!(what, "Khalid found no post en/no-such-post.", "{answer}");
}
