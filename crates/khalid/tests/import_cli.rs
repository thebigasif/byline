//! `khalid import` (KH-API-7): a dry run writes nothing, an import writes
//! the posts with their slugs and stores the redirects the site can answer,
//! and a second import refuses to overwrite.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

/// A site with the demo's settings and no content yet.
#[cfg(test)]
fn site() -> TempDir {
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let temp = tempfile::tempdir().unwrap();
    fs::copy(settings, file(temp.path(), "site.toml")).unwrap();
    fs::create_dir(file(temp.path(), "content")).unwrap();
    temp
}

/// A fixture of the importer crate, by its path inside `fixtures/`.
#[cfg(test)]
fn fixture(name: &str) -> String {
    format!(
        "{}/../khalid-import/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Runs `khalid --site <site>` with `args`, and returns the exit code and
/// what it printed.
#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(args)
        .output()
        .unwrap();
    let mut printed = String::from_utf8(output.stdout).unwrap();
    printed.push_str(&String::from_utf8(output.stderr).unwrap());
    (output.status.code().unwrap(), printed)
}

/// A file inside `site`.
#[cfg(test)]
fn file(site: &Path, path: &str) -> PathBuf {
    let mut file = site.to_path_buf();
    file.push(path);
    file
}

const POST: &str = "content/en/posts/2024-03-12-keep-a-starter-alive.md";

#[test]
fn a_dry_run_reports_and_writes_nothing() {
    let site = site();
    let export = fixture("wordpress/basic.xml");
    let (code, printed) = khalid(site.path(), &["import", "wordpress", &export, "--dry-run"]);
    assert_eq!(code, 0, "{printed}");
    assert!(
        printed.starts_with("This is a dry run: nothing was written."),
        "{printed}"
    );
    assert!(printed.contains("Slugs kept:"), "{printed}");
    assert!(!file(site.path(), "content/en").exists());
    assert!(!file(site.path(), "data").exists());
}

#[test]
fn a_wordpress_import_writes_posts_and_stores_its_redirects() {
    let site = site();
    let export = fixture("wordpress/basic.xml");
    let rules = fixture("wordpress/redirection.json");
    let args = ["import", "wordpress", &export, "--redirection", &rules];
    let (code, printed) = khalid(site.path(), &args);
    assert_eq!(code, 0, "{printed}");
    assert!(!printed.contains("dry run"), "{printed}");
    let post = fs::read_to_string(file(site.path(), POST)).unwrap();
    assert!(post.contains("slug: \"keep-a-starter-alive\""), "{post}");
    let map = fs::read_to_string(file(site.path(), "import/wordpress-redirects.csv")).unwrap();
    let rows = map.lines().count() - 1;
    assert!(rows > 0, "{map}");
    let told = format!("Wrote 7 files and stored {rows} redirects.");
    assert!(printed.starts_with(&told), "{printed}");
    let (code, stored) = khalid(site.path(), &["redirects", "export"]);
    assert_eq!(code, 0, "{stored}");
    assert_eq!(
        stored.lines().count(),
        map.lines().count(),
        "{stored}\n{map}"
    );

    let (code, again) = khalid(site.path(), &["import", "wordpress", &export]);
    assert_eq!(code, 1, "{again}");
    assert!(again.contains(POST), "{again}");
}

#[test]
fn a_ghost_import_answers_in_json() {
    let site = site();
    let export = fixture("ghost/ghost-5.json");
    let args = [
        "--json",
        "import",
        "ghost",
        &export,
        "--old-site",
        "https://old.example.com",
    ];
    let (code, printed) = khalid(site.path(), &args);
    assert_eq!(code, 0, "{printed}");
    let body: Value = serde_json::from_str(&printed).unwrap();
    assert_eq!(body["dry_run"], false);
    let written: Vec<&str> = body["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();
    assert!(written.contains(&"import/ghost-redirects.csv"), "{body}");
    assert!(written.iter().all(|path| file(site.path(), path).is_file()));
}

#[test]
fn a_medium_import_keeps_addresses_on_medium_in_the_file_only() {
    let site = site();
    let posts = file(site.path(), "medium-export/posts");
    fs::create_dir_all(&posts).unwrap();
    let post = file(&posts, "2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html");
    fs::write(post, MEDIUM_POST).unwrap();
    let export = file(site.path(), "medium-export");
    let (code, printed) = khalid(site.path(), &["import", "medium", export.to_str().unwrap()]);
    assert_eq!(code, 0, "{printed}");
    assert!(printed.contains("stored 0 redirects"), "{printed}");
    let map = fs::read_to_string(file(site.path(), "import/medium-redirects.csv")).unwrap();
    assert!(map.contains(",https://www.example.com/blog/"), "{map}");
    assert!(map.contains("https://medium.com/@mara/how-to-bake-rye-bread-3f1a8c4e2b9d,"));
}

#[test]
fn a_substack_import_moves_old_addresses_on_the_same_host() {
    let site = site();
    let posts = file(site.path(), "substack/posts");
    fs::create_dir_all(&posts).unwrap();
    fs::write(
        file(&posts, "140062712.why-rye.html"),
        "<p>Rye needs time.</p>",
    )
    .unwrap();
    let list = "post_id,post_date,is_published,type,audience,title,subtitle\n\
        140062712.why-rye,2024-03-12T09:30:44.123Z,true,newsletter,only_paid,Why rye,\n";
    fs::write(file(site.path(), "substack/posts.csv"), list).unwrap();
    let export = file(site.path(), "substack");
    let (code, printed) = khalid(
        site.path(),
        &["import", "substack", export.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{printed}");
    assert!(
        printed.starts_with("Wrote 2 files and stored 1 redirects."),
        "{printed}"
    );
    let post = fs::read_to_string(file(site.path(), "content/en/posts/2024-03-12-why-rye.md"));
    assert!(post.unwrap().contains("access: \"members\""));
    let (_, explained) = khalid(site.path(), &["redirects", "explain", "/p/why-rye"]);
    assert!(explained.contains("/blog/why-rye/"), "{explained}");
}

#[test]
fn redirects_that_clash_with_stored_ones_are_named() {
    let site = site();
    let clash = file(site.path(), "clash.csv");
    let row = "/keep-a-starter-alive/,/blog/elsewhere/,301";
    fs::write(&clash, format!("from,to,status\n{row}\n")).unwrap();
    let (code, done) = khalid(
        site.path(),
        &["redirects", "import", clash.to_str().unwrap()],
    );
    assert_eq!(code, 0, "{done}");
    let export = fixture("ghost/ghost-5.json");
    let (code, printed) = khalid(site.path(), &["import", "ghost", &export]);
    assert_eq!(code, 1, "{printed}");
    assert!(printed.contains("were not stored"), "{printed}");
    assert!(printed.contains("khalid redirects import import/ghost-redirects.csv"));
}

#[test]
fn an_export_that_is_not_one_imports_nothing() {
    let site = site();
    let export = fixture("ghost/ghost-5.json");
    let (code, printed) = khalid(site.path(), &["import", "wordpress", &export]);
    assert_eq!(code, 1, "{printed}");
    assert!(printed.contains("Nothing was imported"), "{printed}");
    assert!(!file(site.path(), "content/en").exists());
}

const MEDIUM_POST: &str = r#"<html><body><article class="h-entry"><header><h1 class="p-name">How to bake rye bread</h1></header><section data-field="subtitle" class="p-summary">A wetter dough</section><section data-field="body" class="e-content"><section class="section"><div class="section-inner"><h3 class="graf graf--h3 graf--title">How to bake rye bread</h3><p class="graf graf--p">Rye needs time.</p></div></section></section><footer><p>By <a href="https://medium.com/@mara" class="p-author h-card">Mara Lind</a> on <time class="dt-published" datetime="2024-03-12T09:30:44.123Z">then</time>.</p><p><a href="https://medium.com/@mara/how-to-bake-rye-bread-3f1a8c4e2b9d" class="p-canonical">Canonical link</a></p></footer></article></body></html>"#;
