//! The honest update switch (F2) on a running `khalid serve`: an Admin API
//! edit marked `real_update` writes today and its note into the post file,
//! and the served page shows the new updated date. An edit without the
//! switch leaves the date alone.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chrono::Utc;
use serde_json::{Value, json};

/// The demo post every edit here changes.
const ROUTE: &str = "/api/admin/v1/posts/en.why-internal-links-matter/";

/// Its file in the demo site.
const FILE: &str = "content/en/posts/2026-09-14-why-internal-links-matter.md";

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

/// A copy of the demo site.
#[cfg(test)]
fn demo_site() -> tempfile::TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    temp
}

/// A key that may write and publish posts, made by `khalid keys create`.
#[cfg(test)]
fn editor_key(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--name", "Editor", "--scope", "posts:write"])
        .args(["--scope", "posts:publish"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on a free port.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
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
    let address = json["address"].as_str().unwrap().strip_prefix("http://");
    let host = address.unwrap().trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Sends `method` to `path` with `key` and `body`; returns the status and
/// the body as text. Gives up after 20 seconds.
#[cfg(test)]
fn send(server: &Server, key: &str, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
         Authorization: Khalid {key}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n",
        server.host,
        body.len()
    );
    stream
        .write_all(format!("{head}{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, body.to_owned())
}

/// Sends an edit of the demo post; returns the status and the reply.
#[cfg(test)]
fn edit(server: &Server, key: &str, post: &Value) -> (u16, Value) {
    let body = json!({"posts": [post]}).to_string();
    let (status, reply) = send(server, key, "PUT", ROUTE, &body);
    (status, serde_json::from_str(&reply).unwrap_or(Value::Null))
}

/// The served post page, once it holds `wanted`, waiting up to 20 seconds.
#[cfg(test)]
fn page_with(server: &Server, wanted: &str) -> String {
    let mut page = String::new();
    for _ in 0..100 {
        let (_, body) = send(server, "", "GET", "/blog/why-internal-links-matter/", "");
        page = body;
        if page.contains(wanted) {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    page
}

#[test]
fn a_real_update_reaches_the_file_and_the_served_page() {
    let site = demo_site();
    let key = editor_key(site.path());
    let server = start(site.path());
    let shown = page_with(&server, r#"<time datetime="2026-10-20">"#);
    assert!(shown.contains(r#"<time datetime="2026-10-20">"#), "{shown}");

    let (status, post) = send(&server, &key, "GET", ROUTE, "");
    assert_eq!(status, 200, "{post}");
    let post: Value = serde_json::from_str(&post).unwrap();
    let post = &post["posts"][0];
    let title_only = json!({"title": "Why internal links matter so much",
        "updated_at": post["updated_at"]});
    let (status, saved) = edit(&server, &key, &title_only);
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["posts"][0]["updated"], "2026-10-20", "{saved}");

    let markdown = format!(
        "{}\n\nOne more example: a link from a guide.",
        post["markdown"].as_str().unwrap()
    );
    let real = json!({"markdown": markdown,
        "real_update": {"note": "Added a guide example."},
        "updated_at": saved["posts"][0]["updated_at"]});
    let (status, saved) = edit(&server, &key, &real);
    assert_eq!(status, 200, "{saved}");
    let today = Utc::now().date_naive().to_string();
    assert_eq!(saved["posts"][0]["updated"], today.as_str(), "{saved}");

    let mut file = site.path().to_path_buf();
    file.push(FILE);
    let text = fs::read_to_string(file).unwrap();
    assert!(text.contains(&format!("\nupdated: {today}\n")), "{text}");
    assert!(
        text.contains("\nupdated_note: \"Added a guide example.\"\n"),
        "{text}"
    );
    let wanted = format!(r#"<time datetime="{today}">"#);
    let page = page_with(&server, &wanted);
    assert!(page.contains(&wanted), "{page}");
}
