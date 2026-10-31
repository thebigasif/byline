//! The visitor log through `khalid serve` (GEO C1, decision D5): off on a
//! site that does not switch it on; on, one row per visit with a
//! shortened address, and a bot's claim checked against the kept list.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use khalid_analytics::{AnalyticsStore, Visit, Visitor, VisitorLogStore};
use khalid_core::Slug;
use khalid_db::{AnalyticsTable, Database, VisitorLogTable};
use serde_json::Value;

const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0";
const GPTBOT: &str = "Mozilla/5.0 (compatible; GPTBot/1.2; +https://openai.com/gptbot)";
const GOOGLEBOT: &str = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
const POST: &str = "/blog/why-internal-links-matter/";

/// Google's list as the test keeps it. No request leaves the machine:
/// `--test-hosts` leaves every company's host unknown.
const GOOGLE_LIST: &str = r#"{"prefixes":[{"ipv4Prefix":"66.249.64.0/27"}]}"#;

/// A running `khalid serve` on a free port, stopped when dropped.
struct Server(Child, String);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A copy of the demo site in a new folder.
#[cfg(test)]
fn demo() -> tempfile::TempDir {
    let site = tempfile::tempdir().unwrap();
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, site.path()])
        .status();
    assert!(copied.unwrap().success());
    site
}

/// Starts `khalid serve` on `site`, with every outside host unknown, so
/// the address lists are never fetched from the companies.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args([
            "127.0.0.1:0",
            "--json",
            "--test-hosts",
            "nothing.test=127.0.0.1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let address = address.strip_prefix("http://").unwrap();
    Server(child, address.trim_end_matches("/blog/").to_owned())
}

/// Sends a `GET` for `path` with `headers` and returns the status.
#[cfg(test)]
fn get(server: &Server, path: &str, headers: &[(&str, &str)]) -> u16 {
    let mut stream = TcpStream::connect(&server.1).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let mut head = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        server.1
    );
    for (name, value) in headers {
        head = format!("{head}{name}: {value}\r\n");
    }
    stream.write_all(format!("{head}\r\n").as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let raw = String::from_utf8_lossy(&raw).into_owned();
    raw.split(' ').nth(1).unwrap().parse().unwrap()
}

#[cfg(test)]
async fn open(site: &Path) -> (Database, khalid_db::SiteId) {
    let mut file = PathBuf::from(site);
    file.push("data/site.db");
    let db = Database::open(&file).await.unwrap();
    let id = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    (db, id)
}

/// The visitor log once it holds at least `rows`, waiting for the
/// server's next write.
#[cfg(test)]
async fn visits(site: &Path, rows: usize) -> Vec<Visit> {
    let (db, id) = open(site).await;
    let table = VisitorLogTable::new(db.clone(), id);
    for _ in 0..120 {
        let visits = table
            .visits(DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC)
            .await
            .unwrap();
        if visits.len() >= rows {
            db.close().await;
            return visits;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("the server never wrote {rows} visits");
}

/// A visit in one line: address asked, status, short address, and the
/// referrer group or the bot and its check.
#[cfg(test)]
fn row(visit: &Visit) -> String {
    let who = match visit.visitor {
        Visitor::Person(group) => group.as_str().to_owned(),
        Visitor::Bot(bot, check) => format!("{} {}", bot.name(), check.as_str()),
    };
    format!("{} {} {} {who}", visit.path, visit.status, visit.address)
}

#[tokio::test]
async fn a_site_that_does_not_switch_it_on_logs_no_visit() {
    let site = demo();
    let server = start(site.path());
    assert_eq!(get(&server, POST, &[("User-Agent", FIREFOX)]), 200);
    // Wait for the page counts to be written, then one more write.
    let (db, id) = open(site.path()).await;
    let counts = AnalyticsTable::new(db.clone(), id);
    for _ in 0..120 {
        if !counts
            .views(NaiveDate::MIN, NaiveDate::MAX)
            .await
            .unwrap()
            .is_empty()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    tokio::time::sleep(Duration::from_secs(6)).await;
    let log = VisitorLogTable::new(db.clone(), id);
    let all = log.visits(DateTime::<Utc>::MIN_UTC, DateTime::<Utc>::MAX_UTC);
    assert_eq!(all.await.unwrap(), vec![]);
    let mut kept = PathBuf::from(site.path());
    kept.push("data/crawler-ranges");
    assert!(!kept.exists(), "no list is fetched with the log off");
}

#[tokio::test]
async fn a_site_that_switches_it_on_logs_each_visit_with_a_short_address() {
    let site = demo();
    let mut settings = PathBuf::from(site.path());
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, format!("{text}\n[visitor_log]\non = true\n")).unwrap();
    let mut lists = PathBuf::from(site.path());
    lists.push("data/crawler-ranges");
    fs::create_dir_all(&lists).unwrap();
    lists.push("googlebot.json");
    fs::write(&lists, GOOGLE_LIST).unwrap();

    let server = start(site.path());
    let chatgpt = ("Referer", "https://chatgpt.com/");
    assert_eq!(get(&server, POST, &[("User-Agent", FIREFOX), chatgpt]), 200);
    // Once a write is in, the kept list has been read.
    visits(site.path(), 1).await;
    let google = ("X-Forwarded-For", "66.249.64.5");
    get(&server, POST, &[("User-Agent", GOOGLEBOT), google]);
    get(&server, POST, &[("User-Agent", GOOGLEBOT)]);
    get(&server, "/blog/no-such-post/", &[("User-Agent", GPTBOT)]);
    let visits = visits(site.path(), 4).await;

    assert_eq!(
        visits.iter().map(row).collect::<Vec<_>>(),
        [
            format!("{POST} 200 127.0.0.0 ai"),
            format!("{POST} 200 66.249.64.0 Googlebot listed"),
            format!("{POST} 200 127.0.0.0 Googlebot not-listed"),
            "/blog/no-such-post/ 404 127.0.0.0 GPTBot no-list".to_owned(),
        ]
    );
    // The same address with another browser is another visitor.
    assert_ne!(visits[0].code, visits[2].code);
    let mut file = PathBuf::from(site.path());
    file.push("data/site.db");
    drop(server);
    let bytes = fs::read(&file).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("66.249.64.5"));
}
