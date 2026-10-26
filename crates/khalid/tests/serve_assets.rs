//! A folder post's images through `khalid serve`: the image answers at the
//! post's address with its type and tag, the body points at it, and a
//! changed image changes the post's content hash and the tag (KH-CON-3).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use image::{Rgb, RgbImage};
use khalid_content::{ContentTree, read_content};
use serde_json::Value;
use tempfile::TempDir;

/// The demo folder post's chart, inside the site folder.
const CHART: &str = "content/en/posts/2026-10-05-fixing-slow-pages/page-weight.png";

/// Where the chart answers.
const CHART_PATH: &str = "/blog/fixing-slow-pages/page-weight.png";

/// The pause between looks for an edit, and how many looks, 20 seconds in all.
const PAUSE: Duration = Duration::from_millis(100);
const TRIES: u32 = 200;

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

/// One answer: its status, its headers in lower case, and its bytes.
struct Answer {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.head
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}: ")))
    }
}

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn demo_copy() -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
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
    let port = address.strip_prefix("http://127.0.0.1:").unwrap();
    let host = format!("127.0.0.1:{}", port.trim_end_matches("/blog/"));
    Server { child, host }
}

#[cfg(test)]
fn get(server: &Server, path: &str) -> Answer {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(raw[..split].to_vec())
        .unwrap()
        .to_ascii_lowercase();
    Answer {
        status: head.split(' ').nth(1).unwrap().parse().unwrap(),
        head,
        body: raw[split + 4..].to_vec(),
    }
}

/// The content hash of the demo folder post in `site`.
#[cfg(test)]
fn post_hash(site: &Path) -> String {
    let content = read_content(&ContentTree::walk(site).unwrap()).unwrap();
    let post = content
        .posts
        .iter()
        .find(|post| post.path.ends_with("2026-10-05-fixing-slow-pages/index.md"))
        .unwrap();
    post.hash.to_string()
}

/// A different chart: one bar, drawn as a valid PNG.
#[cfg(test)]
fn other_chart() -> Vec<u8> {
    let mut chart = RgbImage::from_pixel(640, 360, Rgb([255, 255, 255]));
    for x in 100..200 {
        for y in 60..320 {
            chart.put_pixel(x, y, Rgb([37, 99, 235]));
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    chart.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn a_folder_post_serves_its_image_and_a_changed_image_changes_hash_and_tag() {
    let site = demo_copy();
    let server = start(site.path());

    let post = get(&server, "/blog/fixing-slow-pages/");
    let html = String::from_utf8(post.body).unwrap();
    assert!(
        html.contains("src=\"/blog/fixing-slow-pages/page-weight.png\""),
        "{html}"
    );

    let chart = get(&server, CHART_PATH);
    assert_eq!(chart.status, 200, "{}", chart.head);
    assert_eq!(chart.header("content-type"), Some("image/png"));
    assert_eq!(chart.body, fs::read(at(site.path(), CHART)).unwrap());
    let old_tag = chart.header("etag").unwrap().to_owned();
    let old_hash = post_hash(site.path());

    fs::write(at(site.path(), CHART), other_chart()).unwrap();
    assert_ne!(
        post_hash(site.path()),
        old_hash,
        "the hash covers the image"
    );
    for _ in 0..TRIES {
        let chart = get(&server, CHART_PATH);
        if chart.header("etag").is_some_and(|tag| tag != old_tag) {
            assert_eq!(chart.body, other_chart());
            return;
        }
        thread::sleep(PAUSE);
    }
    panic!("the changed chart never answered with a new tag");
}
