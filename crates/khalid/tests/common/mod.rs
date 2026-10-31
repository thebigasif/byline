//! A running `khalid serve` on a copy of the demo site, and plain HTTP to
//! it, shared by the Admin API's end-to-end tests.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for a change.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before a wait fails.
const TRIES: u32 = 200;

/// A running `khalid serve`, stopped when dropped.
pub(crate) struct Server {
    child: Child,
    host: String,
    authorization: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A copy of the demo site, so a test can change it.
#[cfg(test)]
pub(crate) fn demo_copy() -> TempDir {
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

/// Runs `khalid --site <site> keys create --json` with `args` and returns
/// the key it showed.
#[cfg(test)]
pub(crate) fn new_key(site: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on `site` on a free port, sending `key` with every
/// Admin API request, and reads the address it prints.
#[cfg(test)]
pub(crate) fn start(site: &Path, key: &str) -> Server {
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
    let authorization = format!("Khalid {key}");
    Server {
        child,
        host,
        authorization,
    }
}

impl Server {
    /// The host and port the server answers on, such as `127.0.0.1:41234`.
    #[cfg(test)]
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    /// Sends `method` to `path` with the key and a JSON `body`, if any, and
    /// returns the status and the body as JSON, or `Null` when it is none.
    #[cfg(test)]
    pub(crate) fn admin(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let auth = [("Authorization", self.authorization.as_str())];
        let (status, body) = self.raw(method, path, &auth, body);
        (status, serde_json::from_str(&body).unwrap_or(Value::Null))
    }

    /// Sends `method` to `path` with `headers` and a JSON `body`, if any,
    /// and returns the status and the body.
    #[cfg(test)]
    pub(crate) fn raw(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&Value>,
    ) -> (u16, String) {
        let body = body.map(Value::to_string).unwrap_or_default();
        let json = (!body.is_empty()).then_some("application/json");
        self.send(method, path, headers, (body.as_bytes(), json))
    }

    /// Sends `method` to `path` with `headers` and `bytes` as they are,
    /// typed as `media_type` when there is one, and returns the status and
    /// the body.
    #[cfg(test)]
    pub(crate) fn send(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        (bytes, media_type): (&[u8], Option<&str>),
    ) -> (u16, String) {
        let host = self.host();
        let mut stream = TcpStream::connect(host).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
            bytes.len()
        );
        if let Some(media_type) = media_type {
            head = format!("{head}Content-Type: {media_type}\r\n");
        }
        for (name, value) in headers {
            head = format!("{head}{name}: {value}\r\n");
        }
        stream.write_all(format!("{head}\r\n").as_bytes()).unwrap();
        stream.write_all(bytes).unwrap();
        // A picture's bytes are not text; only the head and JSON need to be.
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).unwrap();
        let raw = String::from_utf8_lossy(&answer);
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        let status = head.split(' ').nth(1).unwrap().parse().unwrap();
        (status, body.to_owned())
    }

    /// Whether `path` answers `status` within 20 seconds.
    #[cfg(test)]
    pub(crate) fn answers(&self, path: &str, status: u16) -> bool {
        (0..TRIES).any(|_| {
            thread::sleep(PAUSE);
            self.raw("GET", path, &[], None).0 == status
        })
    }
}
