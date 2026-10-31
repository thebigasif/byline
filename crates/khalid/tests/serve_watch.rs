//! The speed and uptime watch through a running `khalid serve` (C6): it
//! looks at the blog's public address, here a stand-in, and a site that
//! stops answering or answers slowly reaches the `alert.raised` webhook.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, Secret, Slug};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::Event;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A running `khalid serve`, stopped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A copy of the demo site whose public address is `public`, the stand-in,
/// with `alerts` added to `[alerts]` and an `alert.raised` webhook there.
#[cfg(test)]
async fn demo_site(public: &MockServer, alerts: &str) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let port = public.address().port();
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap().replace(
        "address = \"https://www.example.com\"",
        &format!("address = \"http://blog.test:{port}\""),
    );
    let alerts = format!("\n[alerts]\nstale_after_days = 0\n{alerts}\n");
    fs::write(&settings, format!("{text}{alerts}")).unwrap();
    let mut path = temp.path().to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let url = AbsoluteUrl::parse(&format!("http://blog.test:{port}/alerts")).unwrap();
    let table = WebhookTable::new(db.clone(), site, khalid_core::SystemClock);
    let _: (_, Secret<String>) = table.create(&url, Event::AlertRaised).await.unwrap();
    db.close().await;
    temp
}

/// Starts `khalid serve` on loopback, looking every second, and waits
/// until it listens.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "blog.test=127.0.0.1"])
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
        .args(["--watch-every", "1"])
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

/// The first alert of `kind` the stand-in received, waiting up to a minute.
#[cfg(test)]
async fn alert_of(public: &MockServer, kind: &str) -> Option<Value> {
    for _ in 0..600 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let received = public.received_requests().await.unwrap();
        let found = received
            .iter()
            .filter(|request| request.url.path() == "/alerts")
            .filter_map(|request| serde_json::from_slice::<Value>(&request.body).ok())
            .find(|body| body["alert"]["kind"] == kind);
        if found.is_some() {
            return found;
        }
    }
    None
}

/// A stand-in public address whose blog home answers `home`.
#[cfg(test)]
async fn public(home: ResponseTemplate) -> MockServer {
    let public = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&public)
        .await;
    Mock::given(method("GET"))
        .and(path("/blog/"))
        .respond_with(home)
        .mount(&public)
        .await;
    public
}

#[tokio::test(flavor = "multi_thread")]
async fn a_public_address_that_fails_three_looks_raises_site_down() {
    let public = public(ResponseTemplate::new(503)).await;
    let site = demo_site(&public, "").await;
    let _server = start(site.path());
    let body = alert_of(&public, "site-down")
        .await
        .expect("a site-down alert");
    let finding = &body["alert"]["findings"][0];
    let port = public.address().port();
    let key = format!("site-down:http://blog.test:{port}/blog/");
    assert_eq!(finding["key"], key);
    let line = finding["line"].as_str().unwrap();
    assert!(line.ends_with("the last look got status 503."), "{line}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_public_address_that_answers_slowly_raises_slow_site() {
    let slow = ResponseTemplate::new(200).set_delay(Duration::from_millis(300));
    let public = public(slow).await;
    let site = demo_site(&public, "slow_ms = 100").await;
    let _server = start(site.path());
    let body = alert_of(&public, "slow-site")
        .await
        .expect("a slow-site alert");
    let line = body["alert"]["findings"][0]["line"].as_str().unwrap();
    assert!(line.ends_with("the limit is 100 ms."), "{line}");
}
