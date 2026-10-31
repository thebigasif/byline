//! Saving the file of a page, glossary term, case study or changelog entry
//! on a running `khalid serve` fires its `.published`, `.published.edited`
//! and `.deleted` webhooks to a stand-in receiver on loopback, signed with
//! each webhook's secret, and a post that names a tag new to the site fires
//! `tag.added` (audit brief G25, KH-API-3).

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The stand-in host for the webhook receiver, on loopback.
const HOSTS: &str = "hooks.test=127.0.0.1";

/// A running `khalid serve`, stopped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A copy of the demo site with a webhook for each of `events` sending to
/// a receiver at `/<event>`, and the running server. Gives back the
/// receiver, the site, the server and each webhook's secret.
#[cfg(test)]
async fn serving(events: &[Event]) -> (MockServer, TempDir, Server, Vec<Secret<String>>) {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let site = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, site.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut file = site.path().to_path_buf();
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let table = WebhookTable::new(db.clone(), id, SystemClock);
    let mut secrets = Vec::new();
    for &event in events {
        let port = receiver.address().port();
        let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/{event}")).unwrap();
        secrets.push(table.create(&url, event).await.unwrap().1);
    }
    db.close().await;
    let server = start(site.path());
    (receiver, site, server, secrets)
}

/// Starts `khalid serve`, reaching the stand-in host on loopback, and
/// waits until it listens.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", HOSTS])
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.contains("address"), "{line}");
    Server(child)
}

/// The one delivery of `event`, waited for up to 20 seconds and checked
/// against `secret`, as JSON.
#[cfg(test)]
async fn the_delivery(receiver: &MockServer, event: Event, secret: &Secret<String>) -> Value {
    let route = format!("/{event}");
    let mut hooks = Vec::new();
    for _ in 0..200 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let all = receiver.received_requests().await.unwrap();
        hooks = all.into_iter().filter(|r| r.url.path() == route).collect();
        if !hooks.is_empty() {
            break;
        }
    }
    assert_eq!(hooks.len(), 1, "{event} fires once");
    let hook = &hooks[0];
    assert_eq!(hook.headers["khalid-event"], event.as_str());
    let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(secret).unwrap());
    verifier
        .verify(signature, &hook.body, &SystemClock)
        .unwrap();
    let body: Value = serde_json::from_slice(&hook.body).unwrap();
    assert_eq!(body["event"], event.as_str(), "{body}");
    body
}

/// Adds the file `name` with `text` to the site, changes it, then removes
/// it, and checks each of `events` fires in turn with the entry under
/// `key`, named `id`.
#[cfg(test)]
async fn add_change_remove(
    events: [Event; 3],
    (name, text): (&str, &str),
    (key, id): (&str, &str),
) {
    let (receiver, site, _server, secrets) = serving(&events).await;
    let mut file = site.path().to_path_buf();
    file.push(name);
    fs::write(&file, text).unwrap();
    let body = the_delivery(&receiver, events[0], &secrets[0]).await;
    assert_eq!(body[key]["id"], id, "{body}");
    assert_eq!(body[key]["file"], name, "{body}");

    fs::write(&file, format!("{text}\nOne more line.\n")).unwrap();
    let body = the_delivery(&receiver, events[1], &secrets[1]).await;
    assert_eq!(body[key]["id"], id, "{body}");

    fs::remove_file(&file).unwrap();
    let body = the_delivery(&receiver, events[2], &secrets[2]).await;
    assert_eq!(body[key]["id"], id, "the entry as it was: {body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_page_fires_page_published_edited_and_deleted() {
    let events = [
        Event::PagePublished,
        Event::PagePublishedEdited,
        Event::PageDeleted,
    ];
    let text = "---\ntitle: Contact Field Notes\nslug: contact\n---\n\nWrite to us.\n";
    let file = ("content/en/pages/contact.md", text);
    add_change_remove(events, file, ("page", "en.contact")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_glossary_term_fires_its_three_events() {
    let events = [
        Event::GlossaryTermPublished,
        Event::GlossaryTermPublishedEdited,
        Event::GlossaryTermDeleted,
    ];
    let text =
        "---\nterm: Crawl budget\nslug: crawl-budget\n---\n\nHow many pages a crawler reads.\n";
    let file = ("content/en/glossary/crawl-budget.md", text);
    add_change_remove(events, file, ("glossary_term", "en.crawl-budget")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_case_study_fires_its_three_events() {
    let events = [
        Event::CaseStudyPublished,
        Event::CaseStudyPublishedEdited,
        Event::CaseStudyDeleted,
    ];
    let text = "---\ntitle: How a cafe got found\nslug: harbour-cafe\ndate: 2026-10-21\n---\n\nIt listed its menu.\n";
    let file = ("content/en/case-studies/harbour-cafe.md", text);
    add_change_remove(events, file, ("case_study", "en.harbour-cafe")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_changelog_entry_fires_its_three_events() {
    let events = [
        Event::ChangelogEntryPublished,
        Event::ChangelogEntryPublishedEdited,
        Event::ChangelogEntryDeleted,
    ];
    let text = "---\ntitle: What changed in November\ndate: 2026-11-02\nversion: 2026.11\n---\n\n- Added a contact page.\n";
    let file = ("content/en/changelog/2026-11.md", text);
    add_change_remove(events, file, ("changelog_entry", "en.2026-11")).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_that_names_a_new_tag_fires_tag_added() {
    let (receiver, site, _server, secrets) = serving(&[Event::TagAdded]).await;
    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-09-14-why-internal-links-matter.md");
    let text = fs::read_to_string(&file).unwrap();
    let tagged = "category: on-page-seo\ntags: [crawl budget]\n";
    fs::write(&file, text.replacen("category: on-page-seo\n", tagged, 1)).unwrap();

    let body = the_delivery(&receiver, Event::TagAdded, &secrets[0]).await;
    assert_eq!(body["tag"]["id"], "en.crawl-budget", "{body}");
    assert_eq!(body["tag"]["name"], "crawl budget", "{body}");
    assert_eq!(body["tag"]["language"], "en", "{body}");
}
