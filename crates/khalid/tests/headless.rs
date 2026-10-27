//! A headless site (`mode = "headless"`, KH-INS-5): `khalid build` writes
//! the feeds, sitemaps and redirects of the demo site and no HTML, and
//! `khalid serve` answers the API, feeds, sitemaps and redirects and 404
//! for every page. The theme is never read, so a broken one stops neither.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// The old address the demo's sitemap post lists in its `aliases`.
const ALIAS: &str = "/blog/sitemaps-explained/";

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A copy of the demo site set to headless, with an alias on one post and
/// theme tokens no theme could read.
#[cfg(test)]
fn headless_demo() -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let settings = at(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", "mode = \"headless\""),
    )
    .unwrap();
    let post = at(
        temp.path(),
        "content/en/posts/2026-09-28-what-a-sitemap-does.md",
    );
    let text = fs::read_to_string(&post).unwrap();
    let aliased = text.replace(
        "translation: sitemap\n",
        &format!("translation: sitemap\naliases:\n  - {ALIAS}\n"),
    );
    fs::write(&post, aliased).unwrap();
    let theme = at(temp.path(), "theme");
    fs::create_dir_all(&theme).unwrap();
    fs::write(at(&theme, "tokens.toml"), "[colors]\nlink = \"red-ish\"\n").unwrap();
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Every file under `folder`, relative to it, sorted.
#[cfg(test)]
fn files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                open.push(path);
            } else {
                let rel = path.strip_prefix(folder).unwrap();
                found.push(rel.to_str().unwrap().to_owned());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_headless_build_writes_the_feeds_sitemaps_and_redirects_and_no_html() {
    let site = headless_demo();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .arg("--out")
        .arg(out.path())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(said.contains("no pages"), "{said}");

    let written = files(out.path());
    let html: Vec<&String> = written
        .iter()
        .filter(|f| {
            Path::new(f)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("html"))
        })
        .collect();
    assert!(html.is_empty(), "{html:?}");
    assert!(
        !written.iter().any(|f| f.starts_with("assets")),
        "{written:?}"
    );
    for file in [
        "sitemap.xml",
        "robots.txt",
        "llms.txt",
        "feed.xml",
        "atom.xml",
        "feed.json",
        "de/feed.xml",
        "_redirects",
    ] {
        assert!(written.iter().any(|f| f == file), "{file} in {written:?}");
    }
    let sitemaps: String = written
        .iter()
        .filter(|f| f.starts_with("sitemap"))
        .map(|f| fs::read_to_string(at(out.path(), f)).unwrap())
        .collect();
    assert!(
        sitemaps.contains("https://www.example.com/blog/what-a-sitemap-does/"),
        "{sitemaps}"
    );
    let redirects = fs::read_to_string(at(out.path(), "_redirects")).unwrap();
    assert!(
        redirects.contains(&format!("{ALIAS} /blog/what-a-sitemap-does/ 301")),
        "{redirects}"
    );
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

/// Starts `khalid serve` on `site` on a free port and reads the address it
/// prints, or fails with what it said instead.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    if line.is_empty() {
        let mut said = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut said)
            .unwrap();
        panic!("serve did not start: {said}");
    }
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

/// A Content API key for `site`, made with `khalid keys create`.
#[cfg(test)]
fn content_key(site: &Path) -> String {
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(["keys", "create", "--json", "--kind", "content"])
        .args(["--name", "Front end"])
        .output()
        .unwrap();
    assert_eq!(made.status.code(), Some(0));
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    made["key"].as_str().unwrap().to_owned()
}

#[test]
fn a_headless_server_answers_the_api_files_and_redirects_and_no_page() {
    let site = headless_demo();
    let key = content_key(site.path());
    let server = start(site.path());

    for page in [
        "/blog/",
        "/blog/what-a-sitemap-does/",
        "/blog/de/",
        "/blog/authors/lena-vogt/",
        "/blog/glossary/topical-authority/",
    ] {
        let (status, head, body) = get(&server, page);
        assert_eq!(status, 404, "{page}");
        assert!(head.contains("content-type: text/html"), "{page}: {head}");
        assert!(body.contains("another app"), "{page}: {body}");
    }
    let files = [
        ("/blog/sitemap.xml", "application/xml", "<sitemapindex"),
        ("/blog/feed.xml", "application/xml", "What a sitemap does"),
        ("/blog/atom.xml", "application/xml", "<feed"),
        ("/blog/de/feed.json", "application/json", "\"version\""),
        ("/blog/robots.txt", "text/plain", "Sitemap:"),
        ("/blog/llms.txt", "text/plain", "Field Notes on SEO"),
    ];
    for (path, media, text) in files {
        let (status, head, body) = get(&server, path);
        assert_eq!(status, 200, "{path}");
        let media = format!("content-type: {media}");
        assert!(head.contains(&media), "{path}: {head}");
        assert!(body.contains(text), "{path}: {body}");
    }
    let (status, head, _) = get(&server, ALIAS);
    assert_eq!(status, 301);
    assert!(
        head.contains("location: /blog/what-a-sitemap-does/"),
        "{head}"
    );

    let api = format!("/api/content/v1/posts/?key={key}&filter=language:en");
    let (status, _, body) = get(&server, &api);
    assert_eq!(status, 200, "{body}");
    let listed: Value = serde_json::from_str(&body).unwrap();
    let slugs: Vec<&str> = listed["posts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|post| post["slug"].as_str())
        .collect();
    assert!(slugs.contains(&"what-a-sitemap-does"), "{slugs:?}");
}
