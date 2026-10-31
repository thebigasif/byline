//! Gated content on a running `khalid serve`, with scripts off (G41, task
//! 5.11): the post shows the note and a sign-up form, the confirmation
//! email's button brings the reader back with the reader cookie, and with
//! that cookie the same address shows the gated part. A subscriber on a
//! new device gets a pass link by email that does the same.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use khalid_mail::StubSmtp;
use serde_json::Value;
use tempfile::TempDir;

/// The post the gated part is added to.
const POST: &str = "/blog/what-a-sitemap-does/";

/// The gated part's body, which only a subscriber reads.
const SECRET: &str = "The full sitemap checklist";

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

/// A copy of the demo site sending mail through `stub`, whose sitemap
/// post ends with gated content.
#[cfg(test)]
fn gated_site(stub: &StubSmtp) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let mut post = temp.path().to_path_buf();
    post.push("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let text = fs::read_to_string(&post).unwrap();
    let gated = format!(
        "\n:::gated{{note=\"Subscribers read the rest.\" label=\"Your email address\" \
         button=\"Read the rest\"}}\n{SECRET}, step by step.\n:::\n"
    );
    fs::write(&post, format!("{text}{gated}")).unwrap();
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let mail = format!(
        "\n[mail]\nfrom = \"news@example.com\"\nname = \"Field Notes\"\n\n[mail.smtp]\n\
         host = \"127.0.0.1\"\nport = {}\nsecurity = \"none\"\nprivate_host = true\n",
        stub.port()
    );
    fs::write(&settings, format!("{text}{mail}")).unwrap();
    temp
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
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Sends a request and returns the status, the head and the body.
#[cfg(test)]
fn send(server: &Server, head: &str, body: &str) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "{head}\r\nHost: {host}\r\nConnection: close\r\n\
         Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_owned(), body.to_owned())
}

/// The value of header `name` in `head`.
#[cfg(test)]
fn header<'h>(head: &'h str, name: &str) -> Option<&'h str> {
    head.lines()
        .filter_map(|line| line.split_once(": "))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

/// The link to `page` in the last email `stub` received, with any line
/// the mail encoding broke joined back up.
#[cfg(test)]
fn link_in(stub: &StubSmtp, page: &str) -> String {
    let message = stub.received().last().unwrap().clone();
    let body = message.replace("=\n", "").replace("=3D", "=");
    let start = body.find(&format!("/blog/{page}?")).unwrap();
    let rest = body.get(start..).unwrap();
    rest.split(char::is_whitespace).next().unwrap().to_owned()
}

/// The reader cookie a `303` sets, checked to bring the reader to [`POST`].
#[cfg(test)]
fn cookie_of(status: u16, head: &str) -> String {
    assert_eq!(status, 303, "{head}");
    assert_eq!(header(head, "location"), Some(POST), "{head}");
    let cookie = header(head, "set-cookie").unwrap();
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    cookie.split(';').next().unwrap().to_owned()
}

/// [`POST`] once the full render behind the first answer has found its
/// gated part, which the locked page then says by varying by cookie.
#[cfg(test)]
fn after_the_full_render(server: &Server) -> (u16, String, String) {
    // A minute at most, asked ten times a second.
    for _ in 0..600 {
        let answer = send(server, &format!("GET {POST} HTTP/1.1"), "");
        if answer.1.contains("vary: cookie") {
            return answer;
        }
        thread::sleep(Duration::from_millis(100));
    }
    send(server, &format!("GET {POST} HTTP/1.1"), "")
}

#[test]
fn a_reader_who_confirms_reads_the_gated_part_with_scripts_off() {
    let stub = StubSmtp::start().unwrap();
    let site = gated_site(&stub);
    let server = start(site.path());
    let (status, head, page) = after_the_full_render(&server);
    assert_eq!(status, 200, "{page}");
    assert!(!page.contains(SECRET), "the public page never shows it");
    assert!(page.contains("Subscribers read the rest."), "{page}");
    let back = format!("<input type=\"hidden\" name=\"back\" value=\"{POST}\" />");
    assert!(page.contains(&back), "{page}");
    assert!(head.contains("vary: cookie"), "{head}");
    let hidden = format!("GET /_unlocked{POST} HTTP/1.1");
    assert_eq!(send(&server, &hidden, "").0, 404);

    let form = "email=grace%40example.com&homepage=&back=%2Fblog%2Fwhat-a-sitemap-does%2F";
    let (status, _, page) = send(&server, "POST /subscribe HTTP/1.1", form);
    assert_eq!(status, 200, "{page}");
    let link = link_in(&stub, "subscribe/confirm");
    let query = link.split_once('?').unwrap().1;
    let (status, _, page) = send(
        &server,
        &format!("GET /blog/subscribe/confirm?{query} HTTP/1.1"),
        "",
    );
    assert_eq!(status, 200, "{page}");
    let token = query.split('&').next().unwrap();
    let button = format!("{token}&back=%2Fblog%2Fwhat-a-sitemap-does%2F");
    assert!(
        page.contains(&format!("name=\"back\" value=\"{POST}\"")),
        "{page}"
    );
    let (status, head, _) = send(&server, "POST /blog/subscribe/confirm HTTP/1.1", &button);
    let cookie = cookie_of(status, &head);

    let opened = format!("GET {POST} HTTP/1.1\r\nCookie: {cookie}");
    let (status, head, page) = send(&server, &opened, "");
    assert_eq!(status, 200, "{head}");
    assert!(page.contains(SECRET), "the cookie opens the gated part");
    assert!(!page.contains("Subscribers read the rest."), "{page}");
    assert_eq!(header(&head, "cache-control"), Some("private, no-store"));
    let forged = format!("GET {POST} HTTP/1.1\r\nCookie: khalid_reader=1.00");
    assert!(!send(&server, &forged, "").2.contains(SECRET));

    let (status, _, _) = send(&server, "POST /subscribe HTTP/1.1", form);
    assert_eq!(status, 200);
    let link = link_in(&stub, "subscribe/pass");
    let query = link.split_once('?').unwrap().1;
    let (status, head, _) = send(
        &server,
        &format!("GET /blog/subscribe/pass?{query} HTTP/1.1"),
        "",
    );
    let again = cookie_of(status, &head);
    let opened = format!("GET {POST} HTTP/1.1\r\nCookie: {again}");
    assert!(send(&server, &opened, "").2.contains(SECRET));
}
