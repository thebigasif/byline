//! `khalid doctor` fetches the blog's public address through `Outbound` and
//! confirms the blog answers there, which proves the proxy rule works (PRD
//! KH-INS-3, KH-OPS-2). A local `wiremock` stand-in plays the public site,
//! so nothing is fetched from the internet (RULES.md BL-TST-3).

use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The host name the binary resolves to the stand-in, through the test hook.
const HOST: &str = "blog.test";

/// The start of a blog home page as Khalid renders it under `/blog/`: it
/// links the site's stylesheet from the mount's `assets/` folder.
const BLOG_HOME: &str = r#"<!doctype html><html lang="en"><head><title>Field Notes on SEO</title>
<link rel="stylesheet" href="/blog/assets/site.3f2a9c1e.css"></head><body><h1>Field Notes on SEO</h1></body></html>"#;

/// The main site's own page, as it answers when no proxy rule sends
/// `/blog/` to Khalid.
const MAIN_SITE: &str = r#"<!doctype html><html lang="en"><head><title>Northwind Bakery</title>
<link rel="stylesheet" href="/style.css"></head><body><h1>Bread</h1></body></html>"#;

/// The demo site's settings with `address`, in a fresh folder.
#[cfg(test)]
fn site_at(address: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let settings = fs::read_to_string(settings)
        .unwrap()
        .replace("https://www.example.com", address);
    fs::write(within(dir.path(), "site.toml"), settings).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Serves `status` and `body` at `/blog/`, and a `robots.txt` that names
/// the blog's sitemap. Returns the server and the site address under
/// [`HOST`].
#[cfg(test)]
async fn stand_in(status: u16, body: &str) -> (MockServer, String) {
    let server = MockServer::start().await;
    let address = format!("http://{HOST}:{}", server.address().port());
    Mock::given(path("/blog/"))
        .respond_with(ResponseTemplate::new(status).set_body_string(body))
        .mount(&server)
        .await;
    let robots = format!("User-agent: *\nAllow: /blog/\n\nSitemap: {address}/blog/sitemap.xml\n");
    Mock::given(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(robots))
        .mount(&server)
        .await;
    (server, address)
}

/// How the binary resolves host names in a test.
#[derive(Clone, Copy)]
enum Names {
    /// Through the system, with the private-network guard as released.
    Real,
    /// [`HOST`] answers on loopback, through the test hook.
    StandIn,
}

/// Runs `khalid --site <site> doctor --json`, off the async thread so the
/// stand-in keeps serving, and returns the exit code and the body.
#[cfg(test)]
async fn doctor(site: &Path, names: Names) -> (Option<i32>, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command.arg("--site").arg(site).args(["doctor", "--json"]);
    if let Names::StandIn = names {
        command.arg("--test-hosts").arg(format!("{HOST}=127.0.0.1"));
    }
    let output: Output = tokio::task::spawn_blocking(move || command.output().unwrap())
        .await
        .unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

/// The address check in a doctor body.
#[cfg(test)]
fn address(body: &Value) -> &Value {
    body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["check"] == "address")
        .unwrap_or_else(|| panic!("no address check in {body}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_blog_answering_under_its_mount_passes_and_exits_0() {
    let (_server, origin) = stand_in(200, BLOG_HOME).await;
    let site = site_at(&origin);
    let (code, body) = doctor(site.path(), Names::StandIn).await;
    assert_eq!(code, Some(0), "{body}");
    let found = address(&body);
    assert_eq!(found["status"], "ok", "{found}");
    assert_eq!(
        found["summary"],
        format!("The blog answers at {origin}/blog/.")
    );
}

/// A running `khalid serve`, stopped when dropped.
struct Served(Child);

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `khalid serve` on `site` on a free port. Returns the server and
/// its port, read from the address it prints once it answers.
#[cfg(test)]
fn serve(site: &Path) -> (Served, u16) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
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
    let port = json["address"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/blog/")
        .parse()
        .unwrap();
    (Served(child), port)
}

/// Copies the folder `from` into `into`.
#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = within(into, entry.file_name().to_str().unwrap());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_recognises_the_demo_blog_as_khalid_serve_answers_it() {
    let demo = tempfile::tempdir().unwrap();
    copy_dir(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/demo-site"
        )),
        demo.path(),
    );
    let (_served, port) = serve(demo.path());
    let site = site_at(&format!("http://{HOST}:{port}"));
    // The served robots.txt names the demo's own host, not the stand-in's,
    // so only the address check is judged here.
    let (_, body) = doctor(site.path(), Names::StandIn).await;
    assert_eq!(address(&body)["status"], "ok", "{body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_main_site_answering_instead_is_a_problem_about_the_proxy_rule() {
    let (_server, origin) = stand_in(200, MAIN_SITE).await;
    let site = site_at(&origin);
    let (code, body) = doctor(site.path(), Names::StandIn).await;
    assert_eq!(code, Some(1), "{body}");
    let problem = &address(&body)["problem"];
    assert_eq!(
        problem["what"],
        format!("{origin}/blog/ answers with a page Khalid did not make.")
    );
    assert!(
        problem["fix"].as_str().unwrap().contains("proxy rule"),
        "{problem}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_error_status_is_a_problem_that_names_it() {
    let (_server, origin) = stand_in(404, "Not Found").await;
    let site = site_at(&origin);
    let (code, body) = doctor(site.path(), Names::StandIn).await;
    assert_eq!(code, Some(1), "{body}");
    assert_eq!(
        address(&body)["problem"]["what"],
        format!("{origin}/blog/ answers with the error 404 instead of the blog.")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_address_nothing_answers_at_is_a_problem() {
    // A port that was free a moment ago: wiremock keeps dropped servers in
    // a pool that still answers, so one cannot stand for silence.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let origin = format!("http://{HOST}:{port}");
    let site = site_at(&origin);
    let (code, body) = doctor(site.path(), Names::StandIn).await;
    assert_eq!(code, Some(1), "{body}");
    assert_eq!(
        address(&body)["problem"]["what"],
        format!("Khalid could not reach {origin}/blog/.")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_address_is_refused_before_anything_is_sent() {
    let site = site_at("http://127.0.0.1:9");
    let (code, body) = doctor(site.path(), Names::Real).await;
    assert_eq!(code, Some(1), "{body}");
    let problem = &address(&body)["problem"];
    assert_eq!(
        problem["what"],
        "http://127.0.0.1:9/blog/ is on a private network, so doctor did not fetch it."
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn broken_settings_skip_the_address_check() {
    let mut site = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    site.push("../../fixtures/invalid/settings-unknown-field");
    let (code, body) = doctor(&site, Names::Real).await;
    assert_eq!(code, Some(1), "{body}");
    let found = address(&body);
    assert_eq!(found["status"], "skipped", "{found}");
    assert!(
        found["summary"].as_str().unwrap().contains("settings"),
        "{found}"
    );
}
