//! The reader script: pages link it under its hash, build writes it, serve
//! answers it and allows it by that hash; with its features off, none.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use tempfile::TempDir;

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

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn demo_copy() -> TempDir {
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
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
    let json: serde_json::Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address
        .strip_prefix("http://")
        .unwrap()
        .replace("/blog/", "");
    Server { child, host }
}

/// Builds a copy of the demo site with `extra` appended to `site.toml`.
#[cfg(test)]
fn build(extra: &str) -> (TempDir, PathBuf) {
    let site = demo_copy();
    let settings = at(site.path(), "site.toml");
    let mut toml = fs::read_to_string(&settings).unwrap();
    toml.push_str(extra);
    fs::write(&settings, toml).unwrap();
    let out = at(site.path(), "public");
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "build", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    (site, out)
}

/// The `src` and `integrity` of the page's script tag, if it has one.
#[cfg(test)]
fn script(html: &str) -> Option<(String, String)> {
    let tag = html.split("<script src=\"").nth(1)?;
    let (src, rest) = tag.split_once('"')?;
    let integrity = rest.split("integrity=\"").nth(1)?.split('"').next()?;
    Some((src.to_owned(), integrity.to_owned()))
}

/// GET `path` from `host`: the head, in lower case names, and the body.
#[cfg(test)]
fn get(host: &str, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    (head.to_ascii_lowercase(), body.to_owned())
}

#[test]
fn a_built_page_links_the_script_the_build_wrote() {
    let (_site, out) = build("");
    let post = fs::read_to_string(at(&out, "what-a-sitemap-does/index.html")).unwrap();
    let (src, _) = script(&post).expect("the post links the reader script");
    assert!(src.starts_with("/blog/assets/reader."), "{src}");
    assert!(post.contains("defer data-copied="), "{post}");
    let file = fs::read_to_string(at(&out, src.trim_start_matches("/blog/"))).unwrap();
    assert!(file.contains("\"use strict\""), "{file}");
    let home = fs::read_to_string(at(&out, "index.html")).unwrap();
    assert_eq!(script(&home).map(|tag| tag.0), Some(src), "the same file");
}

#[test]
fn with_every_feature_it_improves_off_no_page_links_a_script() {
    let (_site, out) = build(
        "\n[reader]\nheading_links = false\nenlarge_images = false\n\
         search_shortcut = false\ntable_of_contents = false\n",
    );
    let post = fs::read_to_string(at(&out, "what-a-sitemap-does/index.html")).unwrap();
    assert_eq!(script(&post), None, "{post}");
    let code = post.matches("<script").count() - post.matches("application/ld+json").count();
    assert_eq!(code, 0, "only JSON-LD data, no code:\n{post}");
}

#[test]
fn serve_answers_the_script_and_its_policy_allows_it_by_its_hash() {
    let site = demo_copy();
    let server = start(site.path());
    let host = &server.host;
    let (head, home) = get(host, "/blog/");
    let (src, integrity) = script(&home).expect("the home page links the reader script");
    let policy = head
        .lines()
        .find_map(|line| line.strip_prefix("content-security-policy: "))
        .expect("a policy");
    let wanted = format!(" '{}'", integrity.to_ascii_lowercase());
    let (sources, _) = policy.split_once("; style-src").unwrap();
    assert!(sources.contains(&wanted), "{policy}");
    let (head, body) = get(host, &src);
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert!(head.contains("content-type: text/javascript"), "{head}");
    assert!(head.contains("immutable"), "cached for a year: {head}");
    assert!(body.contains("\"use strict\""), "{body}");
}
