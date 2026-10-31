//! The editor's SEO panel on a running `khalid serve` (G54): the check
//! route answers a post's score, search snippet and schema graph, and a
//! search title and hand-written JSON-LD saved through the Admin API reach
//! the post file and the next check. A schema template saved through the
//! Admin API reaches `site.toml`, and the check after a restart.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

/// The demo post every request here is about.
const ROUTE: &str = "/api/admin/v1/posts/en.why-internal-links-matter/";

/// Its file in the demo site.
const FILE: &str = "content/en/posts/2026-09-14-why-internal-links-matter.md";

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

/// A copy of the demo site.
#[cfg(test)]
fn demo_site() -> tempfile::TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    temp
}

/// A key that may write posts, change live ones and change the site's
/// settings, made by `khalid keys create`.
#[cfg(test)]
fn writer_key(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--name", "Writer", "--scope", "posts:write"])
        .args(["--scope", "posts:publish", "--scope", "settings:write"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on a free port.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
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

/// Sends `method` to `path` with `key` and `body`; returns the status and
/// the body as JSON. Gives up after 30 seconds.
#[cfg(test)]
fn send(server: &Server, key: &str, (method, path): (&str, &str), body: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
         Authorization: Khalid {key}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n",
        server.host,
        body.len()
    );
    stream
        .write_all(format!("{head}{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// The demo post's check.
#[cfg(test)]
fn check(server: &Server, key: &str) -> Value {
    let route = format!("{ROUTE}check/");
    let (status, json) = send(server, key, ("GET", &route), "");
    assert_eq!(status, 200, "{json}");
    json["checks"][0].clone()
}

/// The `@type` of every node in `schema`'s graph.
#[cfg(test)]
fn types(schema: &Value) -> Vec<String> {
    let nodes = schema["@graph"].as_array().unwrap();
    nodes.iter().map(|node| node["@type"].to_string()).collect()
}

#[test]
fn the_check_follows_a_saved_search_title_and_custom_schema() {
    let site = demo_site();
    let key = writer_key(site.path());
    let server = start(site.path());

    let first = check(&server, &key);
    let score = first["lint"]["score"]["value"].as_u64().unwrap();
    assert!(score <= 100, "{first}");
    let snippet = &first["snippet"];
    let title = snippet["title"].as_str().unwrap();
    assert!(title.starts_with("Why internal links matter"), "{snippet}");
    let url = snippet["url"].as_str().unwrap();
    assert!(
        url.ends_with("/blog/why-internal-links-matter/"),
        "{snippet}"
    );
    let description = snippet["description"].as_str().unwrap();
    assert!(description.starts_with("Internal links help"), "{snippet}");
    assert!(!types(&first["schema"]).contains(&"\"Event\"".to_owned()));

    let (status, post) = send(&server, &key, ("GET", ROUTE), "");
    assert_eq!(status, 200, "{post}");
    let schema = "{\n  \"@type\": \"Event\",\n  \"name\": \"Internal links clinic\",\n  \
                  \"startDate\": \"2026-11-12T18:00:00+00:00\"\n}";
    let settings = json!({"seo_title": "Internal links, plainly {sep} {site}", "schema": schema});
    let edit = json!({"posts": [{"settings": settings,
        "updated_at": post["posts"][0]["updated_at"]}]});
    let (status, saved) = send(&server, &key, ("PUT", ROUTE), &edit.to_string());
    assert_eq!(status, 200, "{saved}");

    let mut file = site.path().to_path_buf();
    file.push(FILE);
    let text = fs::read_to_string(file).unwrap();
    assert!(
        text.contains("\nseo_title: \"Internal links, plainly {sep} {site}\"\n"),
        "{text}"
    );
    let after = check(&server, &key);
    let title = after["snippet"]["title"].as_str().unwrap();
    assert!(title.starts_with("Internal links, plainly "), "{after}");
    assert!(title.ends_with("Field Notes on SEO"), "{after}");
    assert!(
        types(&after["schema"]).contains(&"\"Event\"".to_owned()),
        "{after}"
    );
}

#[test]
fn a_saved_schema_template_reaches_site_toml_and_the_check_after_a_restart() {
    let site = demo_site();
    let key = writer_key(site.path());
    let server = start(site.path());
    let route = "/api/admin/v1/schema-templates/";
    let (status, read) = send(&server, &key, ("GET", route), "");
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["templates"], json!([]), "{read}");
    let collections = read["collections"].as_array().unwrap();
    assert!(collections.contains(&json!("posts")), "{read}");

    let course = json!({"applies_to": "collection", "name": "posts", "type": "Course",
        "properties": {"provider": "{site}"}});
    let change = json!({"templates": [course], "updated_at": read["updated_at"]});
    let (status, saved) = send(&server, &key, ("PUT", route), &change.to_string());
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["restart_needed"], true, "{saved}");
    let mut file = site.path().to_path_buf();
    file.push("site.toml");
    let text = fs::read_to_string(file).unwrap();
    assert!(
        text.contains("[[seo.schema]]\ncollection = \"posts\"\n"),
        "{text}"
    );

    drop(server);
    let server = start(site.path());
    let after = check(&server, &key);
    assert!(
        types(&after["schema"]).contains(&"\"Course\"".to_owned()),
        "{after}"
    );
}
