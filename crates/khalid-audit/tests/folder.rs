//! A built folder answering as a static host would.

use std::fs;
use std::path::Path;

use khalid_audit::{Answer, Folder, Source};
use khalid_core::AbsoluteUrl;

/// Writes `text` to `relative` inside `root`, making its folders.
#[cfg(test)]
fn write(root: &Path, relative: &str, text: &str) {
    let mut file = root.to_path_buf();
    file.push(relative);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

#[cfg(test)]
async fn answer(folder: &Folder, url: &str) -> Answer {
    folder
        .fetch(&AbsoluteUrl::parse(url).unwrap())
        .await
        .unwrap()
}

#[cfg(test)]
fn built() -> (tempfile::TempDir, Folder) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "index.html", "<!doctype html><title>Home</title>");
    write(
        root,
        "why-links/index.html",
        "<!doctype html><title>Why</title>",
    );
    write(
        root,
        "café/index.html",
        "<!doctype html><title>Café</title>",
    );
    write(root, "assets/site.css", "p{}");
    write(root, "_redirects", "/blog/old/ /blog/why-links/ 301\n");
    write(root, "_root/robots.txt", "User-agent: *\n");
    let home = AbsoluteUrl::parse("https://www.example.com/blog/").unwrap();
    let folder = Folder::open(root, &home).unwrap();
    (dir, folder)
}

#[tokio::test]
async fn pages_files_and_redirects_answer_as_a_static_host_does() {
    let (_dir, folder) = built();
    let home = answer(&folder, "https://www.example.com/blog/").await;
    assert_eq!(home.status(), 200);
    assert!(home.is_html());
    let post = answer(&folder, "https://www.example.com/blog/why-links/").await;
    assert_eq!(post.body(), b"<!doctype html><title>Why</title>");
    let css = answer(&folder, "https://www.example.com/blog/assets/site.css").await;
    assert_eq!(css.header("content-type"), Some("text/css; charset=utf-8"));
    let old = answer(&folder, "https://www.example.com/blog/old/").await;
    assert_eq!(old.status(), 301);
    assert_eq!(old.header("location"), Some("/blog/why-links/"));
    let bare = answer(&folder, "https://www.example.com/blog/why-links").await;
    assert_eq!(bare.status(), 301);
    assert_eq!(bare.header("location"), Some("/blog/why-links/"));
    let encoded = answer(&folder, "https://www.example.com/blog/caf%C3%A9/").await;
    assert_eq!(encoded.status(), 200);
}

#[tokio::test]
async fn root_files_come_from_root_and_nothing_leaves_the_folder() {
    let (_dir, folder) = built();
    let robots = answer(&folder, "https://www.example.com/robots.txt").await;
    assert_eq!(robots.body(), b"User-agent: *\n");
    for url in [
        "https://www.example.com/blog/missing/",
        "https://www.example.com/blog/%2E%2E/%2E%2E/etc/passwd",
        "https://www.example.com/shop/",
    ] {
        assert_eq!(answer(&folder, url).await.status(), 404, "{url}");
    }
}

#[test]
fn a_folder_that_is_not_there_is_refused() {
    let home = AbsoluteUrl::parse("https://www.example.com/blog/").unwrap();
    let error = Folder::open(Path::new("/no/such/folder"), &home).unwrap_err();
    assert!(error.to_string().contains("/no/such/folder"));
}
