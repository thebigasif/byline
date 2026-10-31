//! `khalid serve` and `khalid build` agree on a post the publish gate
//! blocks: neither shows it, and both show the rest of the site (KH-LNT-1).
//! Once the post is fixed and saved, the running site shows it.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for a page.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before the test fails.
const TRIES: u32 = 200;

/// A post that passes every block.
const READY: &str = "---
title: Why internal links matter
slug: why-internal-links-matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

An internal link points to [the home page](/blog/).
";

/// A dated post with no author and no internal link: two blocks.
const BLOCKED: &str = "---
title: Choosing link text
slug: choosing-link-text
date: 2026-09-21
---

Good link text names the page it opens.
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

/// `relative` inside `folder`.
#[cfg(test)]
fn at(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}

/// A site with the demo settings, one ready post and one blocked post.
#[cfg(test)]
fn site() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/site.toml");
    let dir = tempfile::tempdir().unwrap();
    fs::copy(&demo, at(dir.path(), "site.toml")).unwrap();
    let posts = at(dir.path(), "content/en/posts");
    fs::create_dir_all(&posts).unwrap();
    let ready = at(&posts, "2026-09-14-why-internal-links-matter.md");
    fs::write(ready, READY).unwrap();
    fs::write(at(&posts, "2026-09-21-choosing-link-text.md"), BLOCKED).unwrap();
    dir
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
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// The status and body of a plain GET of `path`.
#[cfg(test)]
fn get(server: &Server, path: &str) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, body.to_owned())
}

/// Whether `path` answers 200 within the test's patience.
#[cfg(test)]
fn shows(server: &Server, path: &str) -> bool {
    (0..TRIES).any(|_| {
        let found = get(server, path).0 == 200;
        if !found {
            thread::sleep(PAUSE);
        }
        found
    })
}

#[test]
fn serve_leaves_out_the_post_build_leaves_out_and_shows_it_once_fixed() {
    let site = site();
    let folder = site.path().to_str().unwrap();
    let built = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", folder, "build"])
        .output()
        .unwrap();
    assert_eq!(built.status.code(), Some(0));
    let public = at(site.path(), "public");
    assert!(at(&public, "why-internal-links-matter/index.html").exists());
    assert!(!at(&public, "choosing-link-text").exists());

    let server = start(site.path());
    assert!(shows(&server, "/blog/why-internal-links-matter/"));
    let (status, _) = get(&server, "/blog/choosing-link-text/");
    assert_eq!(status, 404, "the blocked post is not live");
    let (_, home) = get(&server, "/blog/");
    assert!(!home.contains("Choosing link text"), "{home}");
    let (_, feed) = get(&server, "/blog/feed.xml");
    assert!(feed.contains("why-internal-links-matter"), "{feed}");
    assert!(!feed.contains("choosing-link-text"), "{feed}");

    let mut fixed = BLOCKED.replace("date:", "author: asif\ndate:");
    fixed.push_str("Read [why internal links matter](/blog/why-internal-links-matter/).\n");
    let file = at(
        site.path(),
        "content/en/posts/2026-09-21-choosing-link-text.md",
    );
    fs::write(file, fixed).unwrap();
    assert!(
        shows(&server, "/blog/choosing-link-text/"),
        "the fixed post never went live"
    );
}
