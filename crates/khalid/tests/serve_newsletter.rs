//! The newsletter on a running `khalid serve`, with scripts off: the plain
//! sign-up form gets a whole page back, at the mount path and where the
//! `::newsletter` block posts, and while the site cannot send email that
//! page says sign-ups are not open and nothing is stored (task 5.5). With
//! `[mail]` set, the confirmation email leaves and its link confirms
//! (task 5.6).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use khalid_mail::StubSmtp;
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

/// A copy of the demo site, so the database it makes stays out of the
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
    let address = json["address"].as_str().unwrap_or_else(|| panic!("{line}"));
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Posts the form `body` to `path` as a browser does, and returns the
/// status and the page.
#[cfg(test)]
fn post_form(server: &Server, path: &str, body: &str) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
         Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, page) = raw.split_once("\r\n\r\n").unwrap();
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        page.to_owned(),
    )
}

#[test]
fn the_plain_form_gets_a_page_saying_sign_ups_are_not_open_yet() {
    let site = demo_copy();
    let server = start(site.path());
    for path in ["/blog/subscribe", "/subscribe"] {
        let (status, page) = post_form(&server, path, "email=grace%40example.com&homepage=");
        assert_eq!(status, 503, "{path}: {page}");
        assert!(
            page.contains("<h1>Signing up is not open yet</h1>"),
            "{page}"
        );
        assert!(page.contains("nothing was saved"), "{page}");
    }
    drop(server);
    let needle = b"grace@example.com";
    for name in ["site.db", "site.db-wal"] {
        let mut file = site.path().to_path_buf();
        file.extend(["data", name]);
        let bytes = fs::read(&file).unwrap_or_default();
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle),
            "{name} holds the address"
        );
    }
}

/// The token in the confirmation link of `message`, with any line the
/// mail encoding broke joined back up.
#[cfg(test)]
fn confirm_token(message: &str) -> String {
    let body = message.replace("=\n", "").replace("=3D", "=");
    let (_, rest) = body.split_once("subscribe/confirm?token=").unwrap();
    rest.split(char::is_whitespace).next().unwrap().to_owned()
}

/// `[mail]` sending through `stub`, a relay on this machine.
#[cfg(test)]
fn mail_settings(stub: &StubSmtp) -> String {
    let port = stub.port();
    format!(
        "\n[mail]\nfrom = \"news@example.com\"\nname = \"Field Notes\"\n\n[mail.smtp]\n\
         host = \"127.0.0.1\"\nport = {port}\nsecurity = \"none\"\nprivate_host = true\n"
    )
}

#[test]
fn with_mail_set_the_confirmation_leaves_and_its_link_confirms() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let mut text = fs::read_to_string(&settings).unwrap();
    text.push_str(&mail_settings(&stub));
    fs::write(&settings, text).unwrap();
    let server = start(site.path());
    let signing_up = "email=grace%40example.com&homepage=";
    let (status, page) = post_form(&server, "/blog/subscribe", signing_up);
    assert_eq!(status, 200, "{page}");
    assert!(page.contains("<h1>Check your inbox</h1>"), "{page}");
    let received = stub.received();
    assert_eq!(received.len(), 1, "{received:?}");
    let message = &received[0];
    assert!(message.contains("To: grace@example.com"), "{message}");
    let from = "From: \"Field Notes\" <news@example.com>";
    assert!(message.contains(from), "{message}");
    let confirming = format!("token={}", confirm_token(message));
    let (status, page) = post_form(&server, "/blog/subscribe/confirm", &confirming);
    assert_eq!(status, 200, "{page}");
    assert!(page.contains("<h1>You are subscribed</h1>"), "{page}");
}

#[test]
fn a_form_joins_a_newsletter_with_a_folder_and_no_other() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let mut text = fs::read_to_string(&settings).unwrap();
    text.push_str(&mail_settings(&stub));
    fs::write(&settings, text).unwrap();
    let mut weekly = site.path().to_path_buf();
    weekly.extend(["content", "newsletters", "weekly"]);
    fs::create_dir_all(&weekly).unwrap();
    let server = start(site.path());
    let joining = "email=grace%40example.com&homepage=&list=weekly";
    let (status, page) = post_form(&server, "/subscribe", joining);
    assert_eq!(status, 200, "{page}");
    assert_eq!(stub.received().len(), 1);
    let made_up = "email=ada%40example.com&homepage=&list=monthly";
    let (status, page) = post_form(&server, "/subscribe", made_up);
    assert_eq!(status, 404, "{page}");
    assert!(
        page.contains("<h1>There is no such newsletter</h1>"),
        "{page}"
    );
    assert_eq!(stub.received().len(), 1, "a made-up list sent an email");
}

#[test]
fn confirming_sends_the_newsletter_s_welcome_email() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let mail = mail_settings(&stub).replace(
        "name = \"Field Notes\"\n",
        "name = \"Field Notes\"\npostal_address = \"1 Park Row, Leeds LS1 5AB, UK\"\n",
    );
    let text = format!("{}{mail}", fs::read_to_string(&settings).unwrap());
    fs::write(&settings, text).unwrap();
    let mut welcome = site.path().to_path_buf();
    welcome.extend(["content", "newsletters", "weekly"]);
    fs::create_dir_all(&welcome).unwrap();
    welcome.push("welcome.md");
    fs::write(
        &welcome,
        "# Welcome to the weekly links\n\nFive links, every Friday.\n",
    )
    .unwrap();
    let server = start(site.path());
    let joining = "email=grace%40example.com&homepage=&list=weekly";
    let (status, page) = post_form(&server, "/subscribe", joining);
    assert_eq!(status, 200, "{page}");
    let confirming = format!("token={}", confirm_token(&stub.received()[0]));
    let (status, page) = post_form(&server, "/blog/subscribe/confirm", &confirming);
    assert_eq!(status, 200, "{page}");
    let received = stub.received();
    assert_eq!(received.len(), 2, "{received:?}");
    let message = &received[1];
    assert!(
        message.contains("Subject: Welcome to the weekly links"),
        "{message}"
    );
    assert!(message.contains("Five links, every Friday."), "{message}");
}
