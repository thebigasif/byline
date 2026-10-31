//! `khalid serve` with the site's own code in `[code]` (G45, KH-SEO-12):
//! the banner is a plain form that works with scripts off, the reader's
//! choice comes back as a cookie that no other site can set, the code runs
//! only for a reader who pressed Allow, under the nonce of the page's
//! policy, and every page type keeps its policy whatever the reader chose.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// The site's code, as an owner would paste a tag manager and a chat.
const CODE: &str = r#"
[code]
head = """
<script async src="https://www.googletagmanager.com/gtag/js?id=G-1"></script>
<script>gtag('config', 'G-1');</script>"""
body = "<script>chat()</script>"
"#;

/// What only the banner says.
const ASK: &str = "It runs only if you allow it.";

/// What only the form to change a choice says.
const CHANGE: &str = "You can change your choice";

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

/// One answer: its status, its head with names lowercased, and its body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    /// The value of the header `name`, or empty.
    #[cfg(test)]
    fn header(&self, name: &str) -> &str {
        let prefix = format!("{name}: ");
        let found = self
            .head
            .lines()
            .find_map(|line| line.strip_prefix(&prefix));
        found.unwrap_or_default()
    }

    /// The nonce the page's policy allows scripts with.
    #[cfg(test)]
    fn nonce(&self) -> &str {
        let policy = self.header("content-security-policy");
        let (_, rest) = policy.split_once("'nonce-").unwrap();
        rest.split_once('\'').unwrap().0
    }
}

/// A copy of the demo site with `extra` added to its settings.
#[cfg(test)]
fn site_with(extra: &str) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, format!("{text}{extra}")).unwrap();
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

/// Sends `method` to `path` with `headers` and a form `body`, as a browser
/// with scripts off does.
#[cfg(test)]
fn send(server: &Server, (method, path): (&str, &str), headers: &[&str], body: &str) -> Answer {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !body.is_empty() {
        request.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
    }
    for header in headers {
        request = format!("{request}{header}\r\n");
    }
    stream
        .write_all(format!("{request}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    Answer {
        status: head.split(' ').nth(1).unwrap().parse().unwrap(),
        head: head.to_ascii_lowercase(),
        body: body.to_owned(),
    }
}

#[cfg(test)]
fn get(server: &Server, path: &str, headers: &[&str]) -> Answer {
    send(server, ("GET", path), headers, "")
}

#[test]
fn the_banner_posts_with_scripts_off_and_the_choice_comes_back_as_a_cookie() {
    let site = site_with(CODE);
    let server = start(site.path());
    let post = "/blog/what-a-sitemap-does/";
    let first = get(&server, post, &[]);
    assert_eq!(first.status, 200);
    assert!(first.body.contains(ASK), "{}", first.body);
    assert!(first.body.contains("action=\"/blog/_consent/\""));
    assert!(!first.body.contains("googletagmanager") && !first.body.contains("chat()"));

    for choice in ["yes", "no"] {
        let form = format!("choice={choice}&back=%2Fblog%2Fwhat-a-sitemap-does%2F");
        let answer = send(&server, ("POST", "/blog/_consent/"), &[], &form);
        assert_eq!(answer.status, 303);
        assert_eq!(answer.header("location"), post);
        let cookie = format!(
            "khalid-consent={choice}; path=/blog/; max-age=15552000; samesite=lax; httponly; secure"
        );
        assert_eq!(answer.header("set-cookie"), cookie);
        assert_eq!(answer.header("cache-control"), "no-store");
        assert!(answer.head.contains("content-security-policy: "));
    }
}

#[test]
fn no_other_site_can_choose_for_a_reader_or_send_them_away() {
    let site = site_with(CODE);
    let server = start(site.path());
    let form = "choice=yes&back=%2Fblog%2F";
    let elsewhere = send(
        &server,
        ("POST", "/blog/_consent/"),
        &["Sec-Fetch-Site: cross-site"],
        form,
    );
    assert_eq!(elsewhere.status, 403);
    assert!(!elsewhere.head.contains("set-cookie"));
    let away = "choice=no&back=https%3A%2F%2Fevil.example%2F";
    let answer = send(&server, ("POST", "/blog/_consent/"), &[], away);
    assert_eq!(answer.status, 303);
    assert_eq!(answer.header("location"), "/blog/");
}

/// Every page type, with the status it answers.
const PAGES: [(&str, u16); 8] = [
    ("/blog/", 200),
    ("/blog/what-a-sitemap-does/", 200),
    ("/blog/about/", 200),
    ("/blog/authors/lena-vogt/", 200),
    ("/blog/glossary/topical-authority/", 200),
    ("/blog/de/warum-interne-links-wichtig-sind/", 200),
    ("/blog/no-such-post/", 404),
    ("/blog/about", 308),
];

#[test]
fn every_page_type_runs_the_code_after_allow_under_its_policy_s_nonce() {
    let site = site_with(CODE);
    let server = start(site.path());
    let unasked = get(&server, "/blog/what-a-sitemap-does/", &[]);
    let policy = unasked.header("content-security-policy");
    assert!(policy.contains("connect-src 'self';"), "{policy}");
    for (path, status) in PAGES {
        let answer = get(&server, path, &["Cookie: khalid-consent=yes"]);
        assert_eq!(answer.status, status, "{path}");
        let nonce = answer.nonce().to_owned();
        assert_eq!(nonce.len(), 32, "{path}: a fresh nonce");
        if status == 308 {
            continue;
        }
        let body = &answer.body;
        let tag = format!(
            "<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-1\" nonce=\"{nonce}\">"
        );
        assert!(body.contains(&tag), "{path}: {body}");
        assert!(body.contains(&format!("<script nonce=\"{nonce}\">chat()</script>")));
        // The banner is cut and the form to change the choice shows, in
        // the page's own language.
        assert!(!body.contains("<!--khalid-ask-->"), "{path}");
        assert!(!body.contains("<!--khalid-chosen-->"), "{path}");
        assert!(body.contains("class=\"consent consent-chosen\""), "{path}");
        let caching = answer.header("cache-control");
        assert_eq!(caching, "private, no-store", "{path}");
    }
    let declined = get(
        &server,
        "/blog/what-a-sitemap-does/",
        &["Cookie: khalid-consent=no"],
    );
    assert!(!declined.body.contains("googletagmanager"));
    assert!(!declined.body.contains(ASK) && declined.body.contains(CHANGE));
}

#[test]
fn code_that_needs_no_consent_runs_on_every_page_with_no_banner() {
    let site = site_with(&format!("{CODE}consent = \"not-needed\"\n"));
    let server = start(site.path());
    for path in ["/blog/what-a-sitemap-does/", "/blog/no-such-post/"] {
        let answer = get(&server, path, &[]);
        let nonce = answer.nonce();
        let script = format!("<script nonce=\"{nonce}\">chat()</script>");
        assert!(answer.body.contains(&script), "{path}: {}", answer.body);
        assert!(!answer.body.contains(ASK), "{path}");
    }
}
