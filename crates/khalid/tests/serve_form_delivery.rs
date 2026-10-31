//! A form answer on a running `khalid serve` reaches the owner's inbox and
//! the `form.submitted` webhook within 5 seconds, through a stand-in mail
//! server and a stand-in receiver on loopback (G40, KH-GRO-1).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_mail::StubSmtp;
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The stand-in host for the webhook receiver, on loopback.
const HOSTS: &str = "hooks.test=127.0.0.1";

/// The demo post the form is added to.
const POST: &str = "/blog/what-a-sitemap-does/";

/// The form block added to the end of [`POST`].
const FORM: &str = "
:::form{id=\"contact\" button=\"Send\"}
- text name: Your name
- email reply_to: Your email address
:::
";

/// What a reader sends, spam trap empty.
const ANSWER: &str = "name=Grace+Hopper&reply_to=grace%40example.com&homepage=";

/// The longest a submission may take to reach both (plan section 10).
const WITHIN: Duration = Duration::from_secs(5);

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

/// A copy of the demo site whose sitemap post ends with [`FORM`], sending
/// mail through a relay on `port` and form answers to `hello@example.com`,
/// with a webhook for `form.submitted` to `receiver`. Returns the site and
/// the webhook's secret.
#[cfg(test)]
async fn demo_site(smtp_port: u16, receiver: &MockServer) -> (TempDir, Secret<String>) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut post_file = temp.path().to_path_buf();
    post_file.extend([
        "content",
        "en",
        "posts",
        "2026-09-28-what-a-sitemap-does.md",
    ]);
    let text = fs::read_to_string(&post_file).unwrap();
    fs::write(&post_file, format!("{text}{FORM}")).unwrap();
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let site = fs::read_to_string(&settings).unwrap();
    let mail = format!(
        "\n[mail]\nfrom = \"news@example.com\"\nforms_to = \"hello@example.com\"\n\n\
         [mail.smtp]\nhost = \"127.0.0.1\"\nport = {smtp_port}\nsecurity = \"none\"\n\
         private_host = true\n"
    );
    fs::write(&settings, [site, mail].concat()).unwrap();

    let mut file = temp.path().to_path_buf();
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let event = Event::FormSubmitted;
    let port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/{event}")).unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    let (_, secret) = table.create(&url, event).await.unwrap();
    db.close().await;
    (temp, secret)
}

/// A receiver that takes every delivery.
#[cfg(test)]
async fn receiver() -> MockServer {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    receiver
}

/// Starts `khalid serve`, reaching the stand-in host on loopback.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", HOSTS])
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
    let address = json["address"].as_str().unwrap().strip_prefix("http://");
    let host = address.unwrap().trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Sends `method` to `path` with `body` as a browser posts a form; returns
/// the status and the page.
#[cfg(test)]
fn send(server: &Server, (method, path): (&str, &str), body: &str) -> (u16, String) {
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
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        page.to_owned(),
    )
}

/// Posts [`ANSWER`] once the post page carries the form; returns the
/// status.
#[cfg(test)]
async fn answer(server: &Server) -> u16 {
    for _ in 0..200 {
        if send(server, ("GET", POST), "")
            .1
            .contains("action=\"/blog/forms/contact\"")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    send(server, ("POST", "/blog/forms/contact"), ANSWER).0
}

/// The `form.submitted` deliveries that reached `receiver`, once there is
/// one.
#[cfg(test)]
async fn delivered(receiver: &MockServer) -> Vec<Request> {
    loop {
        let all = receiver.received_requests().await.unwrap();
        let path = format!("/{}", Event::FormSubmitted);
        let found: Vec<_> = all.into_iter().filter(|r| r.url.path() == path).collect();
        if !found.is_empty() {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// The emails `stub` took, once there is one.
#[cfg(test)]
async fn emailed(stub: &StubSmtp) -> Vec<String> {
    loop {
        let taken = stub.received();
        if !taken.is_empty() {
            return taken;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// The one `form.submitted` delivery in `hooks`, checked against
/// `secret`, as JSON.
#[cfg(test)]
fn the_delivery(hooks: &[Request], secret: &Secret<String>) -> Value {
    assert_eq!(hooks.len(), 1, "form.submitted fires once");
    let hook = &hooks[0];
    let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(secret).unwrap());
    verifier
        .verify(signature, &hook.body, &SystemClock)
        .unwrap();
    serde_json::from_slice(&hook.body).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_reaches_the_inbox_and_the_webhook_within_five_seconds() {
    let (stub, receiver) = (StubSmtp::start().unwrap(), receiver().await);
    let (site, secret) = demo_site(stub.port(), &receiver).await;
    let server = start(site.path());
    assert_eq!(answer(&server).await, 303);
    let both = async { (delivered(&receiver).await, emailed(&stub).await) };
    let within = tokio::time::timeout(WITHIN, both).await;
    let (hooks, emails) = within.expect("the webhook and the email arrive within 5 seconds");

    let body = the_delivery(&hooks, &secret);
    assert_eq!(body["event"], "form.submitted", "{body}");
    let submission = &body["submission"];
    assert_eq!(submission["form"], "contact", "{body}");
    assert_eq!(
        submission["page"], "https://www.example.com/blog/what-a-sitemap-does/",
        "{body}"
    );
    assert_eq!(submission["fields"]["name"], "Grace Hopper", "{body}");
    assert_eq!(
        submission["fields"]["reply_to"], "grace@example.com",
        "{body}"
    );

    assert_eq!(emails.len(), 1, "one email: {emails:?}");
    let email = &emails[0];
    assert!(email.contains("To: hello@example.com"), "{email}");
    assert!(
        email.contains("Subject: New answer to the contact form"),
        "{email}"
    );
    assert!(email.contains("name: Grace Hopper"), "{email}");
    assert!(email.contains("reply_to: grace@example.com"), "{email}");
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_mail_server_down_the_reader_is_thanked_and_the_webhook_fires() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let receiver = receiver().await;
    let (site, secret) = demo_site(port, &receiver).await;
    let server = start(site.path());
    let status = answer(&server).await;
    assert_eq!(status, 303, "the answer is kept though the email fails");

    let within = tokio::time::timeout(WITHIN, delivered(&receiver)).await;
    let hooks = within.expect("the webhook arrives within 5 seconds");
    let body = the_delivery(&hooks, &secret);
    assert_eq!(
        body["submission"]["fields"]["name"], "Grace Hopper",
        "{body}"
    );
}
