//! The form block on a running `khalid serve`, with scripts off: the form
//! posts under the mount path, the route answers it, a reader is thanked
//! and the message is kept in the site database (G4, KH-GRO-1).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The demo post the form is added to.
const POST: &str = "/blog/what-a-sitemap-does/";

/// The form block added to the end of [`POST`].
const FORM: &str = "
:::form{id=\"contact\" button=\"Send\"}
- text name: Your name
- email reply_to: Your email address
:::
";

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

/// A copy of the demo site whose sitemap post ends with [`FORM`].
#[cfg(test)]
fn demo_with_a_form() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let mut post = temp.path().to_path_buf();
    post.extend([
        "content",
        "en",
        "posts",
        "2026-09-28-what-a-sitemap-does.md",
    ]);
    let text = fs::read_to_string(&post).unwrap();
    fs::write(&post, format!("{text}{FORM}")).unwrap();
    temp
}

/// Starts `khalid serve` on `site` on a free port.
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
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Sends `method` to `path` with `body` as a browser posts a form, and
/// returns the status, the head and the page.
#[cfg(test)]
fn send(server: &Server, (method, path): (&str, &str), body: &str) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
         Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, page) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_ascii_lowercase(), page.to_owned())
}

/// The post page once it carries `text`, or the last one seen.
#[cfg(test)]
fn post_page_with(server: &Server, text: &str) -> String {
    let mut page = String::new();
    for _ in 0..200 {
        page = send(server, ("GET", POST), "").2;
        if page.contains(text) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    page
}

#[test]
fn the_demo_form_posts_under_the_mount_and_is_kept() {
    let site = demo_with_a_form();
    let server = start(site.path());
    let action = "action=\"/blog/forms/contact\"";
    let page = post_page_with(&server, action);
    assert!(page.contains(action), "{page}");

    let body = "name=Grace+Hopper&reply_to=grace%40example.com&homepage=";
    let (status, head, _) = send(&server, ("POST", "/blog/forms/contact"), body);
    assert_eq!(status, 303, "{head}");
    assert!(
        head.contains("location: /blog/forms/contact/sent"),
        "{head}"
    );
    let (status, _, thanks) = send(&server, ("GET", "/blog/forms/contact/sent"), "");
    assert_eq!(status, 200, "{thanks}");
    assert!(
        thanks.contains("Thank you. Your message was sent."),
        "{thanks}"
    );
    assert!(thanks.contains(&format!("href=\"{POST}\"")), "{thanks}");

    let (status, _, missing) = send(&server, ("POST", "/blog/forms/survey"), "name=Ada");
    assert_eq!(status, 404, "{missing}");
    assert!(
        missing.contains("This form is not on the site."),
        "{missing}"
    );

    drop(server);
    let needle = b"grace@example.com";
    let kept = ["site.db", "site.db-wal"].iter().any(|name| {
        let mut file = site.path().to_path_buf();
        file.extend(["data", name]);
        let bytes = fs::read(&file).unwrap_or_default();
        bytes.windows(needle.len()).any(|w| w == needle)
    });
    assert!(kept, "the message is not in the site database");
}
