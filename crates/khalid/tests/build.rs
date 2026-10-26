//! `khalid build`: the whole site rendered as HTML into an output folder
//! (RULES.md BL-CLI-1 to BL-CLI-3, BL-FS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn path(folder: &Path) -> &str {
    folder.to_str().unwrap()
}

#[cfg(test)]
fn demo_site() -> PathBuf {
    at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    )
}

#[cfg(test)]
fn page(out: &Path, rel: &str) -> String {
    fs::read_to_string(at(out, rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

#[test]
fn build_renders_every_post_and_home_page_of_the_demo_site() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("22 pages"),
        "{}",
        text(&output.stdout)
    );

    let post = page(out.path(), "why-internal-links-matter/index.html");
    assert!(post.contains("<html lang=\"en\""), "{post}");
    assert!(post.contains("Why internal links matter</h1>"), "{post}");
    assert!(
        post.contains("<h2 id=\"readers-use-them-to-keep-going\""),
        "{post}"
    );
    assert!(post.contains("https://www.example.com/blog/why-internal-links-matter/"));

    let german = page(out.path(), "de/warum-interne-links-wichtig-sind/index.html");
    assert!(german.contains("<html lang=\"de\""), "{german}");

    let home = page(out.path(), "index.html");
    let newest = home.find("/blog/planning-a-topic-cluster/");
    let oldest = home.find("/blog/why-internal-links-matter/");
    assert!(newest.is_some() && newest < oldest, "newest first:\n{home}");
    assert!(page(out.path(), "de/index.html").contains("/blog/de/"));
    assert!(page(out.path(), "404.html").contains("<h1"));
    assert!(page(out.path(), "de/404.html").contains("lang=\"de\""));
}

#[test]
fn build_writes_the_author_and_plain_pages_every_link_points_to() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let post = page(out.path(), "what-a-sitemap-does/index.html");
    assert!(post.contains("/blog/authors/lena-vogt/"), "{post}");
    let author = page(out.path(), "authors/lena-vogt/index.html");
    assert!(author.contains("Lena Vogt</h1>"), "{author}");
    assert!(author.contains("/blog/what-a-sitemap-does/"), "{author}");
    assert!(
        !author.contains("/blog/why-internal-links-matter/"),
        "{author}"
    );
    assert!(page(out.path(), "de/authors/asif/index.html").contains("lang=\"de\""));

    assert!(page(out.path(), "about/index.html").contains("About Field Notes</h1>"));
    assert!(page(out.path(), "de/ueber-uns/index.html").contains("Über Field Notes</h1>"));
}

#[test]
fn build_writes_a_page_for_each_glossary_term_where_its_route_puts_it() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let term = page(out.path(), "glossary/topical-authority/index.html");
    assert!(term.contains("Topical authority</h1>"), "{term}");
    assert!(term.contains("https://www.example.com/blog/glossary/topical-authority/"));
    let german = page(out.path(), "de/glossary/thematische-autoritaet/index.html");
    assert!(german.contains("<html lang=\"de\""), "{german}");
}

#[test]
fn build_turns_each_wikilink_of_the_demo_site_into_a_link_to_its_page() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let post = page(out.path(), "why-internal-links-matter/index.html");
    let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
    assert!(post.contains(link), "{post}");
    let german = page(out.path(), "de/warum-interne-links-wichtig-sind/index.html");
    let link = "<a href=\"/blog/de/glossary/thematische-autoritaet/\">";
    assert!(german.contains(link), "{german}");
    let term = page(out.path(), "glossary/topical-authority/index.html");
    assert!(
        term.contains("<a href=\"/blog/planning-a-topic-cluster/\">"),
        "{term}"
    );
}

#[test]
fn build_draws_the_maths_of_a_post_as_mathml() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    khalid(&["init", site, "--address", "https://www.example.com"]);
    let post = "---\ntitle: Compound growth\nslug: compound-growth\ndate: 2026-09-14\n---\n\n\
                Traffic grows as $v(1+r)^n$ each month.\n";
    fs::write(at(temp.path(), "content/en/posts/compound-growth.md"), post).unwrap();

    let output = khalid(&["--site", site, "build"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let page = page(&at(temp.path(), "public"), "compound-growth/index.html");
    assert!(page.contains("<math"), "{page}");
    assert!(!page.contains("$v(1+r)^n$"), "{page}");
}

#[test]
fn build_stops_at_a_broken_wikilink_and_names_its_file_and_line() {
    let out = tempfile::tempdir().unwrap();
    let site = at(&demo_site(), "../invalid/content-broken-wikilink");
    let output = khalid(&["--site", path(&site), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let said = text(&output.stderr);
    assert!(
        said.contains("why-links-matter.md, line 10: [[link-equity]] matches no post"),
        "{said}"
    );
    assert!(!at(out.path(), "index.html").exists());
}

#[test]
fn build_writes_each_post_where_routes_toml_puts_it() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    let made = khalid(&["init", site, "--address", "https://www.example.com"]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let routes = "[collections.posts]\npermalink = \"/{year}/{slug}/\"\n";
    fs::write(at(temp.path(), "routes.toml"), routes).unwrap();
    let post = "---\ntitle: Dated links\nslug: dated-links\ndate: 2026-09-14\n---\n\nText.\n";
    fs::write(at(temp.path(), "content/en/posts/dated-links.md"), post).unwrap();

    let output = khalid(&["--site", site, "build"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let public = at(temp.path(), "public");
    assert!(page(&public, "2026/dated-links/index.html").contains("Dated links</h1>"));
    assert!(page(&public, "index.html").contains("/2026/dated-links/\""));
}

#[test]
fn build_refuses_a_glossary_term_whose_route_needs_a_date() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    khalid(&["init", site, "--address", "https://www.example.com"]);
    let routes = "[collections.glossary]\npermalink = \"/{year}/{slug}/\"\n";
    fs::write(at(temp.path(), "routes.toml"), routes).unwrap();
    fs::create_dir_all(at(temp.path(), "content/en/glossary")).unwrap();
    let term = "---\nterm: Crawl budget\nslug: crawl-budget\n---\n\nText.\n";
    fs::write(at(temp.path(), "content/en/glossary/crawl-budget.md"), term).unwrap();

    let output = khalid(&["--site", site, "build", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let what = json["error"]["what"].as_str().unwrap();
    assert!(what.contains("crawl-budget.md has no address"), "{what}");
    assert!(!at(temp.path(), "public/index.html").exists());
}

#[test]
fn build_writes_the_stylesheet_every_page_links_with_its_compressed_copies() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let home = page(out.path(), "index.html");
    let (_, rest) = home.split_once(r#"<link rel="stylesheet" href=""#).unwrap();
    let (href, _) = rest.split_once('"').unwrap();
    let file = href.strip_prefix("/blog/").unwrap();
    let hash = file
        .strip_prefix("assets/site.")
        .and_then(|r| r.strip_suffix(".css"));
    assert_eq!(hash.map(str::len), Some(8), "{href}");
    assert!(
        page(out.path(), "de/index.html").contains(href),
        "one stylesheet"
    );
    let css = page(out.path(), file);
    assert!(css.contains("--color-background: #ffffff;"), "{css}");
    for copy in [".br", ".gz"] {
        let size = fs::metadata(at(out.path(), &format!("{file}{copy}")))
            .unwrap()
            .len();
        assert!(size > 0 && size < 20 * 1024, "{copy}: {size} bytes");
    }
}

#[test]
fn init_then_new_then_build_writes_the_post_to_public() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    let made = khalid(&[
        "init",
        site,
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let new = khalid(&["--site", site, "new", "Why links matter"]);
    assert_eq!(new.status.code(), Some(0), "{}", text(&new.stderr));

    let output = khalid(&["--site", site, "build", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let public = at(temp.path(), "public");
    assert_eq!(json["out"], path(&public));
    assert_eq!(json["pages"], 3);
    let post = page(&public, "why-links-matter/index.html");
    assert!(post.contains("Why links matter</h1>"), "{post}");
}

#[test]
fn build_needs_a_site_folder() {
    let output = khalid(&["build"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--site"));
}

#[test]
fn build_refuses_to_write_into_the_content_folder() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    khalid(&["init", site, "--address", "https://www.example.com"]);
    let content = at(temp.path(), "content/html");
    let output = khalid(&["--site", site, "build", "--out", path(&content), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["why"].as_str().unwrap().contains("content"));
    assert!(!content.exists());
}
