//! `khalid serve` answers each page with the head `khalid build` writes for
//! it: canonical, Open Graph, Twitter, hreflang with `x-default` and one
//! JSON-LD graph, on every page type (KH-SEO-2, KH-SEO-3, KH-CON-8).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

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

/// A copy of the demo site, so neither build nor serve writes into the
/// fixture.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
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

/// Builds `site` into a new folder.
#[cfg(test)]
fn build(site: &Path) -> TempDir {
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "build", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    out
}

/// Starts `khalid serve` on `site` on a free port and reads the address it
/// prints.
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

/// The status and body `path` answers with.
#[cfg(test)]
fn get(server: &Server, path: &str) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, body.to_owned())
}

/// The `<head>` of an HTML page.
#[cfg(test)]
fn head(page: &str) -> &str {
    let start = page.find("<head>").unwrap();
    let end = page.find("</head>").unwrap();
    page.get(start..end).unwrap()
}

/// Every HTML page `khalid build` wrote, as the folder-relative path of
/// its file, sorted.
#[cfg(test)]
fn html_files(out: &Path) -> Vec<String> {
    fn walk(base: &Path, folder: &Path, found: &mut Vec<String>) {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, found);
            } else if path.extension().is_some_and(|ext| ext == "html") {
                let rel = path.strip_prefix(base).unwrap();
                found.push(rel.to_str().unwrap().to_owned());
            }
        }
    }
    let mut found = Vec::new();
    walk(out, out, &mut found);
    found.sort();
    found
}

/// The address serve answers a built file at, and the status it answers.
#[cfg(test)]
fn served_at(rel: &str) -> (String, u16) {
    if let Some(folder) = rel.strip_suffix("404.html") {
        return (format!("/blog/{folder}no-such-post/"), 404);
    }
    let folder = rel.trim_end_matches("index.html");
    (format!("/blog/{folder}"), 200)
}

#[cfg(test)]
fn read(out: &Path, rel: &str) -> String {
    let mut file = out.to_path_buf();
    file.push(rel);
    fs::read_to_string(file).unwrap()
}

/// Built pages serve does not answer yet. Serve has one not-found page, the
/// default language's, so a missing German address gets the English one.
const NOT_SERVED_AS_BUILT: [&str; 1] = ["de/404.html"];

#[test]
fn serve_answers_every_built_page_with_the_head_build_wrote() {
    let site = demo_copy();
    let out = build(site.path());
    let server = start(site.path());
    let files = html_files(out.path());
    assert!(files.len() > 10, "{files:?}");
    for rel in &files {
        if NOT_SERVED_AS_BUILT.contains(&rel.as_str()) {
            continue;
        }
        let (path, want) = served_at(rel);
        let (status, body) = get(&server, &path);
        assert_eq!(status, want, "{path}");
        let built = read(out.path(), rel);
        assert_eq!(head(&body), head(&built), "{path} differs from {rel}");
    }
}

/// The pages a reader and a crawler reach, one of each type, in both
/// languages where the demo has both.
const INDEXED: [&str; 9] = [
    "/blog/",
    "/blog/why-internal-links-matter/",
    "/blog/about/",
    "/blog/authors/asif/",
    "/blog/glossary/topical-authority/",
    "/blog/search/",
    "/blog/de/",
    "/blog/de/warum-interne-links-wichtig-sind/",
    "/blog/sitemap/",
];

#[test]
fn every_served_page_type_names_itself_and_has_one_schema_graph() {
    let site = demo_copy();
    let server = start(site.path());
    for path in INDEXED {
        let (status, body) = get(&server, path);
        assert_eq!(status, 200, "{path}");
        let head = head(&body);
        let url = format!("https://www.example.com{path}");
        let tags = [
            format!(r#"<link rel="canonical" href="{url}">"#),
            format!(r#"<meta property="og:url" content="{url}">"#),
            r#"<meta property="og:title" content=""#.to_owned(),
            r#"<meta property="og:type" content=""#.to_owned(),
            r#"<meta name="twitter:card" content=""#.to_owned(),
        ];
        for tag in &tags {
            assert_eq!(
                head.matches(tag.as_str()).count(),
                1,
                "{path} lacks {tag}:\n{head}"
            );
        }
        // A page with no translation names no language version at all.
        if head.contains("hreflang=") {
            let defaults = head.matches(r#"hreflang="x-default""#);
            assert_eq!(defaults.count(), 1, "{path}:\n{head}");
        }
        let graphs = head.matches(r#"<script type="application/ld+json">"#);
        assert_eq!(graphs.count(), 1, "{path}:\n{head}");
        assert!(head.contains(r#""@graph":["#), "{path}:\n{head}");
    }
}

#[test]
fn served_pages_and_their_translations_name_each_other_and_x_default() {
    let site = demo_copy();
    let server = start(site.path());
    let pairs = [
        ("/blog/", "/blog/de/"),
        (
            "/blog/why-internal-links-matter/",
            "/blog/de/warum-interne-links-wichtig-sind/",
        ),
    ];
    for (english, german) in pairs {
        let tags = [
            format!(
                r#"<link rel="alternate" hreflang="en" href="https://www.example.com{english}">"#
            ),
            format!(
                r#"<link rel="alternate" hreflang="de" href="https://www.example.com{german}">"#
            ),
            format!(
                r#"<link rel="alternate" hreflang="x-default" href="https://www.example.com{english}">"#
            ),
        ];
        for path in [english, german] {
            let (_, body) = get(&server, path);
            let head = head(&body);
            for tag in &tags {
                assert!(head.contains(tag.as_str()), "{path} lacks {tag}:\n{head}");
            }
        }
    }
}

#[test]
fn the_served_not_found_page_is_kept_out_of_search() {
    let site = demo_copy();
    let server = start(site.path());
    let (status, body) = get(&server, "/blog/no-such-post/");
    assert_eq!(status, 404);
    let head = head(&body);
    assert!(
        head.contains(r#"<meta name="robots" content="noindex, follow">"#),
        "{head}"
    );
    assert!(!head.contains("hreflang"), "{head}");
}
