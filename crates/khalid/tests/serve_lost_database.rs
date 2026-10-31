//! `khalid serve` on a site whose `data/site.db` was deleted: it makes the
//! database again from the files, writes the index back, and answers every
//! page type, the search and the Content API (KH-CON-10, ADR 0001, RULES.md
//! BL-FS-1). Keys are not content, so a key made before the loss is gone.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use khalid_core::{Locale, Slug};
use khalid_db::Database;
use serde_json::Value;

/// The pause between looks for the index.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before the test fails.
const TRIES: u32 = 200;

/// The database and the two files `SQLite` keeps beside it.
const DATABASE: [&str; 3] = ["data/site.db", "data/site.db-wal", "data/site.db-shm"];

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

/// `rel` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, rel: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(rel);
    path
}

/// Copies the folder `from` into `into`. A `data` folder is left out:
/// tests that build the demo site in place leave one behind.
#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "data" {
            continue;
        }
        let target = within(into, entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `khalid --site <site>` with `args`.
#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(args)
        .output()
        .unwrap()
}

/// A new Content API key for `site`, made with `khalid keys create`.
#[cfg(test)]
fn content_key(site: &Path) -> String {
    let made = khalid(
        site,
        &[
            "keys", "create", "--json", "--kind", "content", "--name", "Web",
        ],
    );
    assert_eq!(made.status.code(), Some(0));
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    made["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on `site` on a free port and reads the address it
/// prints.
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

/// Fetches `path` and returns the status and the body.
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
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        body.to_owned(),
    )
}

/// How many posts of each language the index in `site`'s database holds.
#[cfg(test)]
fn indexed(site: &Path) -> usize {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let db = Database::open(&within(site, "data/site.db")).await.unwrap();
        let main = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let mut count = 0;
        for language in ["en", "de"] {
            let locale = Locale::try_new(language).unwrap();
            count += db.posts(main, &locale, "posts", 100).await.unwrap().len();
        }
        db.close().await;
        count
    })
}

/// Waits until `path` answers with `text` in its body: the index is
/// written in the background after the server starts.
#[cfg(test)]
fn once_it_shows(server: &Server, path: &str, text: &str) -> (u16, String) {
    let mut answer = get(server, path);
    for _ in 0..TRIES {
        if answer.1.contains(text) {
            break;
        }
        thread::sleep(PAUSE);
        answer = get(server, path);
    }
    answer
}

#[test]
fn serve_makes_a_deleted_database_again_and_answers_every_page_type() {
    let temp = tempfile::tempdir().unwrap();
    let site = within(temp.path(), "demo-site");
    copy_dir(
        &within(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            "../../fixtures/demo-site",
        ),
        &site,
    );
    let rebuilt = khalid(&site, &["db", "rebuild"]);
    assert_eq!(rebuilt.status.code(), Some(0));
    let lost_key = content_key(&site);
    for name in DATABASE {
        let _ = fs::remove_file(within(&site, name));
    }
    assert!(!within(&site, "data/site.db").exists());

    let server = start(&site);
    assert!(
        within(&site, "data/site.db").exists(),
        "serve made it again"
    );
    let pages = [
        ("/blog/", 200, "Field Notes on SEO"),
        ("/blog/what-a-sitemap-does/", 200, "What a sitemap does"),
        ("/blog/about/", 200, "About Field Notes</h1>"),
        ("/blog/authors/lena-vogt/", 200, "Lena Vogt</h1>"),
        ("/blog/authors/", 200, "<h1>Authors</h1>"),
        ("/blog/archive/", 200, "<h1>Archive</h1>"),
        (
            "/blog/category/technical-seo/",
            200,
            "<h1>Technical SEO</h1>",
        ),
        (
            "/blog/glossary/topical-authority/",
            200,
            "Topical authority</h1>",
        ),
        ("/blog/de/", 200, "lang=\"de\""),
        (
            "/blog/de/warum-interne-links-wichtig-sind/",
            200,
            "lang=\"de\"",
        ),
        ("/blog/sitemap.xml", 200, "<sitemapindex"),
        ("/blog/feed.xml", 200, "<rss"),
        ("/blog/no-such-post/", 404, "<h1"),
    ];
    for (path, status, text) in pages {
        let (answered, body) = get(&server, path);
        assert_eq!(answered, status, "{path}");
        assert!(body.contains(text), "{path}:\n{body}");
    }

    let link = r#"<a href="/blog/what-a-sitemap-does/">"#;
    let (status, body) = once_it_shows(&server, "/blog/search/?q=sitemaps", link);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(link), "the search index came back: {body}");
    assert_eq!(indexed(&site), 10, "the index came back whole");

    let posts = "/api/content/v1/posts/?filter=language:en&key=";
    let (status, _) = get(&server, &format!("{posts}{lost_key}"));
    assert_eq!(
        status, 401,
        "a key lives only in the database it was made in"
    );
    let (status, body) = get(&server, &format!("{posts}{}", content_key(&site)));
    assert_eq!(status, 200, "{body}");
    let listed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(listed["meta"]["pagination"]["total"], 5, "{listed}");
}
