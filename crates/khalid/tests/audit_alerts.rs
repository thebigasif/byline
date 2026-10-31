//! `khalid audit` raises the broken links it finds to the `alert.raised`
//! webhook, once while they last (KH-GRO-6, G44).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{AbsoluteUrl, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::Event;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A copy of the demo site with a link to a missing page and an
/// `alert.raised` webhook to `receiver`.
#[cfg(test)]
async fn broken_site(receiver: &MockServer) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut post = temp.path().to_path_buf();
    post.push("content/en/posts/2026-09-14-why-internal-links-matter.md");
    let text = fs::read_to_string(&post).unwrap();
    let broken = text.replacen(
        "Start with one link per post.",
        "Start with one link per post, such as [our old guide](/blog/old-guide/).",
        1,
    );
    assert_ne!(
        text, broken,
        "the sentence to change is gone from the demo post"
    );
    fs::write(&post, broken).unwrap();
    let mut path = temp.path().to_path_buf();
    path.push("data");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let receiver_port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{receiver_port}/alerts")).unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    table.create(&url, Event::AlertRaised).await.unwrap();
    db.close().await;
    temp
}

/// Runs `khalid audit --offline` on `site`, reaching the receiver on
/// loopback.
#[cfg(test)]
fn audit(site: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "hooks.test=127.0.0.1"])
        .args(["audit", "--offline"])
        .output()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_audit_raises_a_broken_link_once_by_webhook() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let site = broken_site(&receiver).await;
    for _ in 0..2 {
        let output = audit(site.path());
        let said = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{said}");
    }
    let hooks = receiver.received_requests().await.unwrap();
    assert_eq!(
        hooks.len(),
        1,
        "the second audit finds the same link and sends nothing"
    );
    let body: Value = serde_json::from_slice(&hooks[0].body).unwrap();
    let alert = &body["alert"];
    assert_eq!(body["event"], "alert.raised");
    assert_eq!(alert["kind"], "broken-links");
    assert_eq!(
        alert["fix"],
        "Run khalid audit to see each link, then fix or remove it."
    );
    let line = alert["findings"][0]["line"].as_str().unwrap();
    assert!(
        line.contains("https://www.example.com/blog/old-guide/"),
        "{line}"
    );
}
