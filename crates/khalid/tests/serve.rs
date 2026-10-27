//! `khalid serve`: the demo site served over HTTP, every page type with its
//! Content Security Policy, an edited post shown without a restart, a
//! graceful stop, and an address it cannot listen on (RULES.md BL-SEC-2,
//! BL-RUN-10, BL-CLI-3). The first test is task 1.15's done line.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for an edit.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before the test fails.
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

/// One answer: its status, its Content Security Policy, and its body.
struct Answer {
    status: u16,
    policy: String,
    body: String,
}

/// A copy of the demo site, so a test can edit a post.
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
    let host = address.strip_prefix("http://127.0.0.1:").unwrap();
    let host = format!("127.0.0.1:{}", host.trim_end_matches("/blog/"));
    Server { child, host }
}

/// Fetches `path` with a plain HTTP/1.1 request.
#[cfg(test)]
fn get(server: &Server, path: &str) -> Answer {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let policy = head
        .lines()
        .find_map(|line| line.strip_prefix("content-security-policy: "))
        .unwrap_or_default();
    Answer {
        status: head.split(' ').nth(1).unwrap().parse().unwrap(),
        policy: policy.to_owned(),
        body: body.to_owned(),
    }
}

#[test]
fn serve_answers_every_page_type_with_a_csp_and_shows_an_edit_without_a_restart() {
    let site = demo_copy();
    let server = start(site.path());
    let pages = [
        ("/blog/", 200, "Field Notes on SEO"),
        ("/blog/what-a-sitemap-does/", 200, "What a sitemap does"),
        ("/blog/about/", 200, "About Field Notes</h1>"),
        ("/blog/authors/lena-vogt/", 200, "Lena Vogt</h1>"),
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
        ("/blog/no-such-post/", 404, "<h1"),
        ("/blog/about", 308, ""),
    ];
    let mut policies = Vec::new();
    for (path, status, text) in pages {
        let answer = get(&server, path);
        assert_eq!(answer.status, status, "{path}");
        assert!(answer.body.contains(text), "{path}:\n{}", answer.body);
        let policy = answer.policy;
        assert!(policy.contains("'nonce-"), "{path}: {policy}");
        assert!(policy.contains("frame-src https://"), "{path}: {policy}");
        policies.push(policy);
    }
    policies.sort();
    policies.dedup();
    assert_eq!(policies.len(), pages.len(), "a fresh nonce every time");

    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let post = fs::read_to_string(&file).unwrap();
    let edited = post.replace(
        "A sitemap is a file,",
        "A sitemap, saved while serving, is a file,",
    );
    fs::write(&file, edited).unwrap();
    let shown = (0..TRIES).any(|_| {
        thread::sleep(PAUSE);
        get(&server, "/blog/what-a-sitemap-does/")
            .body
            .contains("saved while serving")
    });
    assert!(shown, "the edit never showed");
}

/// Asks for `path` with Brotli and returns the head, lowercased, and the body size.
#[cfg(test)]
fn head_of(server: &Server, path: &str) -> (String, usize) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nAccept-Encoding: br\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..end]).to_ascii_lowercase();
    (head, raw.len() - end - 4)
}

#[test]
fn serve_answers_the_stylesheet_the_home_page_links() {
    let site = demo_copy();
    let server = start(site.path());
    let home = get(&server, "/blog/").body;
    let (_, rest) = home.split_once(r#"<link rel="stylesheet" href=""#).unwrap();
    let (href, _) = rest.split_once('"').unwrap();
    let hash = href
        .strip_prefix("/blog/assets/site.")
        .and_then(|rest| rest.strip_suffix(".css"))
        .unwrap_or_else(|| panic!("{href}"));
    assert!(
        hash.len() == 8 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "{href}"
    );

    let css = get(&server, href);
    assert_eq!(css.status, 200, "{href}");
    assert!(css.body.contains("--color-link:"), "{}", css.body);
    // Pages answer plain at once and get their Brotli copies in the
    // background, so look again until the copy is there.
    let mut answer = head_of(&server, href);
    for _ in 0..TRIES {
        if answer.0.contains("content-encoding: br") {
            break;
        }
        thread::sleep(PAUSE);
        answer = head_of(&server, href);
    }
    let (head, brotli_bytes) = answer;
    assert!(head.contains("content-type: text/css"), "{head}");
    assert!(head.contains("content-encoding: br"), "{head}");
    assert!(head.contains("max-age=31536000, immutable"), "{head}");
    assert!(brotli_bytes < css.body.len() / 2, "{brotli_bytes} bytes");
}

#[test]
fn serve_puts_the_site_tokens_into_the_stylesheet() {
    let site = demo_copy();
    let mut theme = site.path().to_path_buf();
    theme.push("theme");
    fs::create_dir_all(&theme).unwrap();
    theme.push("tokens.toml");
    fs::write(&theme, "[colors]\nlink = \"#b3261e\"\n").unwrap();
    let server = start(site.path());
    let home = get(&server, "/blog/").body;
    let (_, rest) = home.split_once(r#"<link rel="stylesheet" href=""#).unwrap();
    let (href, _) = rest.split_once('"').unwrap();
    let css = get(&server, href).body;
    assert!(css.contains("--color-link: #b3261e;"), "{css}");
}

#[test]
fn serve_refuses_tokens_it_cannot_read_and_names_the_file() {
    let site = demo_copy();
    let mut theme = site.path().to_path_buf();
    theme.push("theme");
    fs::create_dir_all(&theme).unwrap();
    theme.push("tokens.toml");
    fs::write(&theme, "[colors]\nlink = \"red-ish\"\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "serve"])
        .args(["--listen", "127.0.0.1:0"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("theme/tokens.toml"), "{said}");
    assert!(said.contains("line 2"), "{said}");
}

#[test]
fn serve_stops_and_names_an_image_it_cannot_render_after_answering_first() {
    let site = demo_copy();
    let mut image = site.path().to_path_buf();
    image.push("content/en/posts/2026-10-05-fixing-slow-pages/page-weight.png");
    fs::write(&image, "not a picture").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "serve"])
        .args(["--listen", "127.0.0.1:0"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "it stops, it does not hang");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("page-weight.png"), "{said}");
}

#[test]
fn serve_answers_the_sitemaps_feeds_and_aliases_beside_the_pages() {
    let site = demo_copy();
    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let post = fs::read_to_string(&file).unwrap();
    let alias = "translation: sitemap\naliases:\n  - /blog/sitemaps-explained/\n";
    fs::write(&file, post.replace("translation: sitemap\n", alias)).unwrap();
    let server = start(site.path());
    let moved = get(&server, "/blog/sitemaps-explained/");
    assert_eq!(moved.status, 301, "the alias moved to its post");
    for (path, text) in [
        ("/blog/sitemap.xml", "<sitemapindex"),
        ("/blog/feed.xml", "<rss"),
        ("/blog/de/atom.xml", "<feed"),
        ("/blog/robots.txt", "Sitemap:"),
    ] {
        let answer = get(&server, path);
        assert_eq!(answer.status, 200, "{path}");
        assert!(answer.body.contains(text), "{path}:\n{}", answer.body);
    }
}

#[test]
fn serve_answers_the_admin_app_at_admin_with_its_own_policy() {
    let site = demo_copy();
    let server = start(site.path());
    let admin = get(&server, "/admin/");
    // CI builds Khalid without running npm, so the binary may hold no admin
    // and say how to add it; with admin/dist built, it is the app's page.
    match admin.status {
        200 => assert!(admin.body.contains("<div id=\"root\">"), "{}", admin.body),
        503 => assert!(admin.body.contains("npm run build"), "{}", admin.body),
        other => panic!("/admin/ answered {other}"),
    }
    assert!(
        admin.policy.contains("script-src 'self'"),
        "{}",
        admin.policy
    );
    assert!(
        admin.policy.contains("frame-ancestors 'none'"),
        "{}",
        admin.policy
    );
    let blog = get(&server, "/blog/");
    assert_eq!(blog.status, 200, "the pages still answer beside the admin");
}

#[test]
fn serve_answers_the_home_page_then_stops_gracefully_when_told_to() {
    let site = demo_copy();
    let mut server = start(site.path());
    let home = get(&server, "/blog/");
    assert_eq!(home.status, 200);
    assert!(home.body.contains("Field Notes on SEO"), "{}", home.body);
    assert!(home.policy.contains("'nonce-"), "{}", home.policy);
    let pid = server.child.id().to_string();
    let told = Command::new("kill").args(["-TERM", &pid]).status().unwrap();
    assert!(told.success());
    let status = server.child.wait().unwrap();
    assert_eq!(status.code(), Some(0), "a stop on request is a success");
}

#[test]
fn serve_refuses_an_address_it_cannot_listen_on() {
    let site = demo_copy();
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = taken.local_addr().unwrap().to_string();
    let site = site.path().to_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site, "serve", "--listen", &address])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains(&address), "{said}");
    assert!(said.contains("--listen"), "names the fix: {said}");
}
