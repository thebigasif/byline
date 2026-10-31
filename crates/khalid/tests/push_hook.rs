//! Git mode's push hook on a running `khalid serve`: a push signed with
//! `KHALID_GIT_HOOK_SECRET` makes the blog pull, and the merged change is
//! served; an unsigned one is refused (KH-EDT-4, plan section 4.6).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The demo post the writer changes.
const POST: &str = "content/en/posts/2026-09-21-how-to-write-a-title-tag.md";

/// The secret the hook checks, made up for this test.
const SECRET: &str = "made-for-this-test-3f9c0b5e2a8d4c71";

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
fn within(folder: &Path, relative: &str) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push(relative);
    path
}

#[cfg(test)]
fn git(folder: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Writer",
            "-c",
            "user.email=writer@example.com",
        ])
        .arg("-C")
        .arg(folder)
        .args(args)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "git {args:?}: {said}");
}

/// A bare repository, a writer's clone holding the demo site, and the
/// blog's own clone, in that order.
#[cfg(test)]
fn repos() -> (TempDir, TempDir, TempDir) {
    let origin = tempfile::tempdir().unwrap();
    git(origin.path(), &["init", "--bare", "--quiet", "-b", "main"]);
    let writer = tempfile::tempdir().unwrap();
    let demo = within(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site/.",
    );
    let copied = Command::new("cp")
        .arg("-R")
        .arg(&demo)
        .arg(writer.path())
        .status();
    assert!(copied.unwrap().success());
    fs::write(within(writer.path(), ".gitignore"), "data/\npublic/\n").unwrap();
    let origin_path = origin.path().to_str().unwrap();
    git(writer.path(), &["init", "--quiet", "-b", "main"]);
    git(writer.path(), &["add", "."]);
    git(writer.path(), &["commit", "--quiet", "-m", "The demo site"]);
    git(writer.path(), &["remote", "add", "origin", origin_path]);
    git(writer.path(), &["push", "--quiet", "-u", "origin", "main"]);
    let blog = tempfile::tempdir().unwrap();
    git(blog.path(), &["clone", "--quiet", origin_path, "."]);
    (origin, writer, blog)
}

/// Starts `khalid serve` on `site` with the hook's secret set.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args(["127.0.0.1:0", "--json"])
        .env("KHALID_GIT_HOOK_SECRET", SECRET)
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
    let host = address
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches("/blog/")
        .to_owned();
    Server { child, host }
}

/// Sends `method` to `path` with `headers`, and returns the status and body.
#[cfg(test)]
fn request(
    server: &Server,
    (method, path): (&str, &str),
    headers: &[(&str, &str)],
) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let body = if method == "POST" {
        "{\"ref\":\"refs/heads/main\"}"
    } else {
        ""
    };
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head = format!("{head}{name}: {value}\r\n");
    }
    stream
        .write_all(format!("{head}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        body.to_owned(),
    )
}

#[test]
fn a_signed_push_is_pulled_and_served_and_an_unsigned_one_is_refused() {
    let (_origin, writer, blog) = repos();
    let server = start(blog.path());
    let page = ("GET", "/blog/how-to-write-a-title-tag/");
    assert_eq!(request(&server, page, &[]).0, 200);
    // The file watch is set up off the runtime after the first answer, and
    // a change made before it is ready waits for the next one.
    thread::sleep(Duration::from_secs(3));
    let post = within(writer.path(), POST);
    let before = fs::read_to_string(&post).unwrap();
    fs::write(
        &post,
        format!("{before}\nA paragraph merged through git.\n"),
    )
    .unwrap();
    git(
        writer.path(),
        &["commit", "--quiet", "-am", "Merge the new paragraph"],
    );
    git(writer.path(), &["push", "--quiet"]);

    let hook = ("POST", "/api/hooks/git");
    let (status, said) = request(&server, hook, &[("X-Gitlab-Token", "a-wrong-secret")]);
    assert_eq!(status, 401, "{said}");
    assert!(said.contains("KHALID_GIT_HOOK_SECRET"), "{said}");
    let (status, said) = request(&server, hook, &[("X-Gitlab-Token", SECRET)]);
    assert_eq!(status, 202, "{said}");
    let shown = (0..200).any(|_| {
        thread::sleep(Duration::from_millis(100));
        request(&server, page, &[])
            .1
            .contains("A paragraph merged through git.")
    });
    let pulled = fs::read_to_string(within(blog.path(), POST)).unwrap();
    assert!(
        pulled.contains("A paragraph merged through git."),
        "the blog pulled"
    );
    assert!(shown, "the pushed paragraph is served within 20 seconds");
}
