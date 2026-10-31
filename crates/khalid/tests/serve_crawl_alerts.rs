//! The crawl alerts through a running `khalid serve` (GEO C4): with the
//! visitor log on, Googlebot unseen for a week reaches the `alert.raised`
//! webhook. With the log off, nothing is raised.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chrono::{DateTime, Days, Utc};
use khalid_analytics::{
    BotCheck, Crawler, ShortAddress, Visit, Visitor, VisitorKey, VisitorLogStore,
};
use khalid_core::{AbsoluteUrl, Clock, Slug, SystemClock};
use khalid_db::{Database, VisitorLogTable, WebhookTable};
use khalid_webhooks::Event;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A running `khalid serve`, stopped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A copy of the demo site with `extra` settings, an `alert.raised`
/// webhook to `receiver`, and Googlebot's last listed fetch 8 days ago.
#[cfg(test)]
async fn demo_site(receiver: &MockServer, extra: &str) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let mut file = OpenOptions::new().append(true).open(&settings).unwrap();
    writeln!(file, "\n{extra}").unwrap();
    let mut path = temp.path().to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/alerts")).unwrap();
    let hooks = WebhookTable::new(db.clone(), site, SystemClock);
    hooks.create(&url, Event::AlertRaised).await.unwrap();
    let log = VisitorLogTable::new(db.clone(), site);
    let googlebot = googlebot(SystemClock.now() - Days::new(8));
    log.save(&[googlebot], DateTime::<Utc>::MIN_UTC)
        .await
        .unwrap();
    db.close().await;
    temp
}

#[cfg(test)]
fn googlebot(at: DateTime<Utc>) -> Visit {
    let address: IpAddr = "66.249.66.1".parse().unwrap();
    let bot = Crawler::named("Googlebot").unwrap();
    Visit {
        at,
        path: "/blog/why-internal-links-matter/".to_owned(),
        status: 200,
        address: ShortAddress::of(address),
        code: VisitorKey::from_bytes([5; 32]).code(address, None),
        visitor: Visitor::Bot(bot, BotCheck::Listed),
    }
}

/// Starts `khalid serve` with a check every second. Only the receiver's
/// host is known, so no address list is fetched.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "hooks.test=127.0.0.1"])
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
        .args(["--alert-every", "1"])
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

/// The kinds of every alert delivered after waiting `seconds`.
#[cfg(test)]
async fn kinds_after(receiver: &MockServer, seconds: u64) -> Vec<Value> {
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    let hooks = receiver.received_requests().await.unwrap();
    hooks
        .iter()
        .map(|hook| serde_json::from_slice::<Value>(&hook.body).unwrap())
        .map(|body| body["alert"].clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn googlebot_unseen_for_a_week_is_raised_while_the_visitor_log_is_on() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let site = demo_site(&receiver, "[visitor_log]\non = true").await;
    let _server = start(site.path());
    let mut stopped = Vec::new();
    for _ in 0..30 {
        stopped = kinds_after(&receiver, 1).await;
        stopped.retain(|alert| alert["kind"] == "crawl-stopped");
        if !stopped.is_empty() {
            break;
        }
    }
    assert_eq!(stopped.len(), 1, "{stopped:?}");
    let finding = &stopped[0]["findings"][0];
    let line = finding["line"].as_str().unwrap();
    assert!(
        line.starts_with("Googlebot last fetched a page on "),
        "{line}"
    );
    assert!(line.ends_with(", 8 days ago."), "{line}");
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_about_crawling_is_raised_while_the_visitor_log_is_off() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let site = demo_site(&receiver, "").await;
    let _server = start(site.path());
    let alerts = kinds_after(&receiver, 4).await;
    assert!(
        alerts.iter().all(|alert| alert["kind"] != "crawl-stopped"),
        "{alerts:?}"
    );
}
