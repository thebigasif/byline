//! Server analytics through `khalid serve` (KH-GRO-4): plain requests with
//! no scripts and no cookies are counted by where the reader came from,
//! and bots land in the crawl log instead.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chrono::NaiveDate;
use khalid_analytics::{AiTool, AnalyticsStore, CrawlerKind, Crawls, ReferrerGroup, Views};
use khalid_core::Slug;
use khalid_db::{AnalyticsTable, Database};
use serde_json::Value;

const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0";
const GPTBOT: &str = "Mozilla/5.0 (compatible; GPTBot/1.2; +https://openai.com/gptbot)";
const GOOGLEBOT: &str = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
const POST: &str = "/blog/why-internal-links-matter/";

/// A running `khalid serve` on a free port, stopped when dropped.
struct Server(Child, String);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args(["127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    Server(child, address.trim_end_matches("/blog/").to_owned())
}

/// Sends `method` to `path` with `headers`, as a browser with scripts off
/// would, and returns the status and the head in lower case.
#[cfg(test)]
fn send(server: &Server, (method, path): (&str, &str), headers: &[(&str, &str)]) -> (u16, String) {
    let mut stream = TcpStream::connect(&server.1).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        server.1
    );
    for (name, value) in headers {
        head = format!("{head}{name}: {value}\r\n");
    }
    stream.write_all(format!("{head}\r\n").as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let raw = String::from_utf8_lossy(&raw).to_lowercase();
    let head = raw
        .split_once("\r\n\r\n")
        .map_or(raw.as_ref(), |(head, _)| head);
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        head.to_owned(),
    )
}

/// The views and crawls in the site's database once `ready` holds them,
/// waiting for the server's next write.
#[cfg(test)]
async fn stored(
    site: &Path,
    ready: impl Fn(&[Views], &[Crawls]) -> bool,
) -> (Vec<Views>, Vec<Crawls>) {
    let mut file = PathBuf::from(site);
    file.push("data/site.db");
    let db = Database::open(&file).await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let table = AnalyticsTable::new(db.clone(), site);
    let (first, last) = (NaiveDate::MIN, NaiveDate::MAX);
    for _ in 0..120 {
        let views = table.views(first, last).await.unwrap();
        let crawls = table.crawls(first, last).await.unwrap();
        if ready(&views, &crawls) {
            db.close().await;
            return (views, crawls);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("the server never wrote its analytics");
}

#[tokio::test]
async fn readers_with_scripts_off_are_counted_by_source_and_bots_are_logged() {
    let site = tempfile::tempdir().unwrap();
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, site.path()])
        .status();
    assert!(copied.unwrap().success());
    let server = start(site.path());
    let ua = ("User-Agent", FIREFOX);
    let (status, head) = send(
        &server,
        ("GET", POST),
        &[ua, ("Referer", "https://chatgpt.com/")],
    );
    assert_eq!(status, 200);
    assert!(!head.contains("set-cookie"), "{head}");
    send(
        &server,
        ("GET", &format!("{POST}?utm_source=chatgpt.com")),
        &[ua],
    );
    send(
        &server,
        ("GET", POST),
        &[ua, ("Referer", "https://www.google.com/")],
    );
    send(
        &server,
        ("GET", POST),
        &[ua, ("Referer", "https://www.perplexity.ai/search")],
    );
    // Not views: a HEAD, a prefetch, a 404, and a script.
    send(&server, ("HEAD", POST), &[ua]);
    send(&server, ("GET", POST), &[ua, ("Sec-Purpose", "prefetch")]);
    send(&server, ("GET", "/blog/no-such-post/"), &[ua]);
    send(&server, ("GET", POST), &[("User-Agent", "curl/8.5.0")]);
    send(&server, ("GET", POST), &[("User-Agent", GPTBOT)]);
    send(
        &server,
        ("GET", "/blog/no-such-post/"),
        &[("User-Agent", GOOGLEBOT)],
    );
    let (views, crawls) = stored(site.path(), |views, crawls| {
        views.iter().map(|row| row.views).sum::<u64>() >= 4 && crawls.len() >= 3
    })
    .await;
    let counted: Vec<_> = views
        .iter()
        .map(|row| (row.path.as_str(), row.group, row.views))
        .collect();
    assert_eq!(
        counted,
        [
            (POST, ReferrerGroup::Search, 1),
            (POST, ReferrerGroup::Ai(AiTool::ChatGpt), 2),
            (POST, ReferrerGroup::Ai(AiTool::Perplexity), 1)
        ]
    );
    let logged: Vec<_> = crawls
        .iter()
        .map(|row| {
            (
                row.path.as_str(),
                row.crawler.as_str(),
                row.kind,
                row.last_status,
            )
        })
        .collect();
    assert_eq!(
        logged,
        [
            ("/blog/no-such-post/", "Googlebot", CrawlerKind::Search, 404),
            (POST, "GPTBot", CrawlerKind::Ai, 200),
            (POST, "Other bot", CrawlerKind::Other, 200),
        ]
    );
}
