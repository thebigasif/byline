//! `khalid doctor` fetches the host's own `robots.txt` through `Outbound`
//! and confirms it names the blog's sitemap, since search engines read it
//! only at the root of the host (KH-SEO-5, KH-OPS-2, ADR "root files go in
//! their own folder"). A local `wiremock` stand-in plays the main site.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The host name the binary resolves to the stand-in, through the test hook.
const HOST: &str = "blog.test";

/// The main site's `robots.txt` with the lines from the blog's `_root/`.
const WITH_SITEMAP: &str =
    "User-agent: *\nDisallow: /checkout/\n\nSitemap: {origin}/blog/sitemap.xml\n";

/// The main site's `robots.txt` before anyone added the blog's lines.
const WITHOUT_SITEMAP: &str =
    "User-agent: *\nDisallow: /checkout/\n\nSitemap: {origin}/sitemap.xml\n";

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// The demo site's settings with `address` and `mount`, in a fresh folder.
#[cfg(test)]
fn site_at(address: &str, mount: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let settings = fs::read_to_string(settings)
        .unwrap()
        .replace("https://www.example.com", address)
        .replace("mount = \"/blog/\"", &format!("mount = \"{mount}\""));
    fs::write(within(dir.path(), "site.toml"), settings).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// Serves `status` and `body` at `/robots.txt`, with `{origin}` in the body
/// replaced by the stand-in's address under [`HOST`]. Returns the server
/// and that address.
#[cfg(test)]
async fn main_site(status: u16, body: &str) -> (MockServer, String) {
    let server = MockServer::start().await;
    let origin = format!("http://{HOST}:{}", server.address().port());
    Mock::given(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(status).set_body_string(body.replace("{origin}", &origin)),
        )
        .mount(&server)
        .await;
    (server, origin)
}

/// Runs `khalid --site <site> doctor --json` with [`HOST`] on loopback, off
/// the async thread so the stand-in keeps serving, and returns the robots
/// check from the body.
#[cfg(test)]
async fn robots_check(site: &Path) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command.arg("--site").arg(site).args(["doctor", "--json"]);
    command.arg("--test-hosts").arg(format!("{HOST}=127.0.0.1"));
    let output: Output = tokio::task::spawn_blocking(move || command.output().unwrap())
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["check"] == "robots")
        .unwrap_or_else(|| panic!("no robots check in {body}"))
        .clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_robots_txt_naming_the_blogs_sitemap_passes() {
    let (_server, origin) = main_site(200, WITH_SITEMAP).await;
    let site = site_at(&origin, "/blog/");
    let found = robots_check(site.path()).await;
    assert_eq!(found["status"], "ok", "{found}");
    assert_eq!(
        found["summary"],
        format!("{origin}/robots.txt names the blog's sitemap.")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_robots_txt_without_the_blogs_sitemap_is_a_problem() {
    let (_server, origin) = main_site(200, WITHOUT_SITEMAP).await;
    let site = site_at(&origin, "/blog/");
    let problem = &robots_check(site.path()).await["problem"];
    assert_eq!(
        problem["what"],
        format!("{origin}/robots.txt does not name the blog's sitemap.")
    );
    let fix = problem["fix"].as_str().unwrap();
    assert!(
        fix.contains(&format!("Sitemap: {origin}/blog/sitemap.xml")),
        "{fix}"
    );
    assert!(fix.contains("_root"), "{fix}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_host_with_no_robots_txt_is_a_problem() {
    let (_server, origin) = main_site(404, "Not Found").await;
    let site = site_at(&origin, "/blog/");
    let problem = &robots_check(site.path()).await["problem"];
    assert_eq!(
        problem["what"],
        format!("{origin}/robots.txt answers with the error 404.")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blog_at_the_root_of_its_host_skips_the_check() {
    let (_server, origin) = main_site(404, "Not Found").await;
    let site = site_at(&origin, "/");
    let found = robots_check(site.path()).await;
    assert_eq!(found["status"], "skipped", "{found}");
}
