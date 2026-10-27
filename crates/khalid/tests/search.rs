//! Site search end to end: `khalid serve` answers `/search/?q=` on the demo
//! site with ranked results and the matching words marked, as plain HTML
//! that needs no script, in each language (plan Phase 5, KH-WEB-2).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// How long the index may take to be written after the server starts.
const INDEXED_WITHIN: Duration = Duration::from_secs(20);

#[cfg(test)]
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

/// A copy of the demo site, so its database is made fresh.
#[cfg(test)]
fn demo() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

/// A running `khalid serve`, stopped when dropped.
struct Server {
    child: Child,
    host: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts `khalid serve` on `site` on a free port.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
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
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Fetches `path` and returns the status, the head, lowercased, and the body.
#[cfg(test)]
fn get(server: &Server, path: &str) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_ascii_lowercase(), body.to_owned())
}

/// The results page for `path` once it lists `listed`: the index is written
/// in the background after the server starts.
#[cfg(test)]
fn results(server: &Server, path: &str, listed: &str) -> (u16, String, String) {
    let tries = INDEXED_WITHIN.as_millis() / 100;
    let mut answer = get(server, path);
    for _ in 0..tries {
        if answer.2.contains(listed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
        answer = get(server, path);
    }
    answer
}

#[test]
fn a_search_lists_ranked_marked_results_with_no_script_and_escapes_the_query() {
    let site = demo();
    let server = start(site.path());

    let link = r#"<a href="/blog/what-a-sitemap-does/">"#;
    let (status, head, body) = results(&server, "/blog/search/?q=sitemaps", link);
    assert_eq!(status, 200, "{body}");
    assert!(head.contains("content-type: text/html"), "{head}");
    assert!(head.contains("content-security-policy:"), "{head}");
    assert!(body.contains("Results for “sitemaps”"), "{body}");
    let first = body.split("<li>").nth(1).unwrap();
    assert!(first.starts_with(&format!("\n<h3>{link}")), "{body}");
    assert!(body.contains("<mark>sitemap</mark>"), "{body}");
    assert!(!body.contains("<script>"), "the results need no script");

    let (_, _, german) = results(&server, "/blog/de/search/?q=Sitemap", "/blog/de/");
    assert!(german.contains("Ergebnisse für „Sitemap“"), "{german}");
    assert!(!german.contains(link));

    // While typing, the reader script asks for the same results as JSON,
    // the last word matched as the start of a word.
    let (status, head, json) = get(&server, "/blog/search/?q=sitem&typing=1");
    assert_eq!(status, 200, "{json}");
    assert!(head.contains("content-type: application/json"), "{head}");
    let found: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(found["heading"], "Results for “sitem”");
    assert_eq!(found["results"][0]["href"], "/blog/what-a-sitemap-does/");
    let snippet = found["results"][0]["snippet"].as_array().unwrap();
    assert!(snippet.iter().any(|part| part["kind"] == "match"), "{json}");

    let (status, _, body) = get(&server, "/blog/search/?q=%3Cb%3Eunicorn");
    assert_eq!(status, 200);
    assert!(body.contains("Results for “&lt;b&gt;unicorn”"), "{body}");
    assert!(body.contains("Nothing matched."), "{body}");
    let (status, _, body) = get(&server, "/blog/search/");
    assert_eq!(status, 200);
    assert!(body.contains(r#"<form class="search""#));
    assert!(!body.contains("search-results"), "{body}");
}

#[test]
fn a_static_build_writes_the_lock_card_and_a_small_index_per_language() {
    let site = demo();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", "mode = \"static\""),
    )
    .unwrap();
    let built = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");

    let read = |rel: &str| {
        let mut file = site.path().to_path_buf();
        file.push("public");
        file.push(rel);
        fs::read_to_string(&file).unwrap_or_else(|error| panic!("{rel}: {error}"))
    };
    let page = read("search/index.html");
    assert!(page.contains("Search needs a running server"), "{page}");
    let index: Value = serde_json::from_str(&read("search/index.json")).unwrap();
    assert_eq!(index["language"], "en");
    let pages = index["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 5, "the five English posts: {index}");
    let sitemap = pages
        .iter()
        .find(|page| page["href"] == "/blog/what-a-sitemap-does/")
        .unwrap();
    let terms = sitemap["terms"].as_array().unwrap();
    assert!(terms.contains(&Value::from("sitemap")), "{sitemap}");
    let german: Value = serde_json::from_str(&read("de/search/index.json")).unwrap();
    assert_eq!(german["language"], "de");
}
