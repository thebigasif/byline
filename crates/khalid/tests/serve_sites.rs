//! `khalid serve` on a sites folder (plan 4.4, PRD KH-EDT-7): one process
//! answers each site on its own domain, a super admin signed in on the
//! home site adds a site that answers without a restart, and the panel is
//! closed to everyone else, a signed-in owner of another site included.

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

/// The pause between looks, and how many looks before a wait fails.
const PAUSE: Duration = Duration::from_millis(100);
const TRIES: u32 = 300;

/// A running `khalid serve`, stopped when dropped.
struct Server {
    child: Child,
    address: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One answer: the status, the head, and the body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn folder(root: &Path, domain: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push("sites");
    path.push(domain);
    path
}

/// The sign-in token of a new owner with `email` of the site in `site`.
#[cfg(test)]
fn owner_token(site: &Path, email: &str) -> String {
    let args = [
        "--json", "staff", "owner", "--email", email, "--name", "Asif",
    ];
    let made = khalid(site, &args);
    assert_eq!(made.status.code(), Some(0), "{made:?}");
    let json: Value = serde_json::from_slice(&made.stdout).unwrap();
    let link = json["link"].as_str().unwrap();
    link.split_once("#token=").unwrap().1.to_owned()
}

#[cfg(test)]
fn start(root: &Path) -> (Server, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", root.to_str().unwrap(), "serve", "--listen"])
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
    let address = address.strip_prefix("http://").unwrap().to_owned();
    (Server { child, address }, json)
}

impl Server {
    /// Sends `method` to `path` on `host` with `headers` and a JSON body.
    #[cfg(test)]
    fn ask(
        &self,
        host: &str,
        (method, path): (&str, &str),
        headers: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Answer {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let body = body.map(Value::to_string).unwrap_or_default();
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            write!(head, "{name}: {value}\r\n").unwrap();
        }
        stream
            .write_all(format!("{head}\r\n{body}").as_bytes())
            .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        Answer {
            status,
            head: head.to_owned(),
            body: body.to_owned(),
        }
    }

    /// Whether the home page of `host` answers 200 within 30 seconds.
    #[cfg(test)]
    fn answers(&self, host: &str) -> bool {
        (0..TRIES).any(|_| {
            thread::sleep(PAUSE);
            self.ask(host, ("GET", "/"), &[], None).status == 200
        })
    }

    /// Redeems a sign-in `token` on `host`: the cookie pair and CSRF token.
    #[cfg(test)]
    fn sign_in(&self, host: &str, token: &str) -> (String, String) {
        let path = "/api/admin/v1/session/link/redeem/";
        let answer = self.ask(host, ("POST", path), &[], Some(&json!({ "token": token })));
        assert_eq!(answer.status, 200, "{}", answer.body);
        let cookie = answer.head.lines().find_map(|line| {
            let (name, value) = line.split_once(": ")?;
            name.eq_ignore_ascii_case("set-cookie").then_some(value)
        });
        let pair = cookie.unwrap().split(';').next().unwrap();
        let json: Value = serde_json::from_str(&answer.body).unwrap();
        (pair.to_owned(), json["csrf"].as_str().unwrap().to_owned())
    }
}

#[test]
fn one_process_serves_each_site_and_a_super_admin_adds_one_without_a_restart() {
    let root = tempfile::tempdir().unwrap();
    for args in [
        &["sites", "add", "example.com"][..],
        &["sites", "add", "news.example.com"],
        &["sites", "admin", "owner@example.com"],
    ] {
        let done = khalid(root.path(), args);
        assert_eq!(done.status.code(), Some(0), "{args:?}: {done:?}");
    }
    let home_token = owner_token(&folder(root.path(), "example.com"), "owner@example.com");
    let news_token = owner_token(
        &folder(root.path(), "news.example.com"),
        "owner@example.com",
    );

    let (server, said) = start(root.path());
    assert_eq!(said["sites"], json!(["example.com", "news.example.com"]));
    for host in ["example.com", "news.example.com"] {
        assert!(server.answers(host), "{host} never answered");
    }
    let unknown = server.ask("shop.example.com", ("GET", "/"), &[], None);
    assert_eq!(unknown.status, 404);
    assert!(
        unknown.body.contains("No site on this server"),
        "{}",
        unknown.body
    );

    let (cookie, csrf) = server.sign_in("example.com", &home_token);
    let home = [
        ("Cookie", cookie.as_str()),
        ("x-khalid-csrf", csrf.as_str()),
    ];
    let sites = ("GET", "/api/admin/v1/sites/");
    let listed = server.ask("example.com", sites, &home, None);
    assert_eq!(listed.status, 200, "{}", listed.body);
    let listed: Value = serde_json::from_str(&listed.body).unwrap();
    assert_eq!(
        listed["sites"][1],
        json!({ "domain": "news.example.com", "home": false })
    );

    // The home site's session means nothing on another site, and the same
    // person signed in as owner there is no super admin there.
    assert_eq!(
        server.ask("news.example.com", sites, &home, None).status,
        401
    );
    let (cookie, csrf) = server.sign_in("news.example.com", &news_token);
    let news = [
        ("Cookie", cookie.as_str()),
        ("x-khalid-csrf", csrf.as_str()),
    ];
    let refused = server.ask(
        "news.example.com",
        ("POST", sites.1),
        &news,
        Some(&json!({ "domain": "shop.example.com" })),
    );
    assert_eq!(refused.status, 403, "{}", refused.body);
    assert!(!folder(root.path(), "shop.example.com").exists());

    let shop = json!({ "domain": "shop.example.com" });
    let added = server.ask("example.com", ("POST", sites.1), &home, Some(&shop));
    assert_eq!(added.status, 201, "{}", added.body);
    assert!(
        server.answers("shop.example.com"),
        "the added site never answered"
    );
    let listed = khalid(root.path(), &["sites", "list", "--json"]);
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed["sites"][2]["domain"], "shop.example.com");

    // One stop signal stops every site and the listener, and the process
    // ends well, within 8 seconds: well inside the 10 seconds grace a
    // request in flight may take.
    let mut server = server;
    let pid = server.child.id().to_string();
    let sent = Command::new("kill").args(["-TERM", &pid]).status().unwrap();
    assert!(sent.success());
    let ended = (0..80).find_map(|_| {
        thread::sleep(PAUSE);
        server.child.try_wait().unwrap()
    });
    assert_eq!(ended.and_then(|status| status.code()), Some(0), "{ended:?}");
}
