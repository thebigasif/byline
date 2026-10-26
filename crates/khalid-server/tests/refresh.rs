//! Pages are rendered off the async runtime and replaced as a whole, so an
//! edit shows without a restart and a broken edit never takes the site down.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use khalid_server::{
    LivePages, MediaType, Page, PagePath, Render, Rendered, SecurityHeaders, ServerError,
    render_pages, router,
};
use tower::ServiceExt;

/// Renders each file in a folder as the page `/<file name>/`, the way the
/// real renderer turns each post file into its page.
#[derive(Debug)]
struct Folder(PathBuf);

#[cfg(test)]
impl Render for Folder {
    type Error = std::io::Error;

    fn render(&self) -> Result<Rendered, Self::Error> {
        let mut pages = Vec::new();
        for entry in fs::read_dir(&self.0)? {
            let path = entry?.path();
            let text = fs::read_to_string(&path)?;
            if text.contains("{{ broken") {
                return Err(std::io::Error::other("a template does not close"));
            }
            let name = path.file_stem().unwrap().to_str().unwrap();
            let address = PagePath::try_new(&format!("/{name}/")).unwrap();
            pages.push(Page::new(address, MediaType::Html, text));
        }
        Ok(Rendered::new(pages, "<h1>Page not found</h1>"))
    }
}

#[cfg(test)]
fn under(folder: &Path, name: &str) -> PathBuf {
    let mut path = folder.to_owned();
    path.push(name);
    path
}

#[cfg(test)]
fn site(post: &str) -> (tempfile::TempDir, Arc<Folder>) {
    let folder = tempfile::tempdir().unwrap();
    edit(folder.path(), post);
    let renderer = Arc::new(Folder(folder.path().to_owned()));
    (folder, renderer)
}

#[cfg(test)]
fn edit(folder: &Path, post: &str) {
    fs::write(under(folder, "what-a-sitemap-does.md"), post).unwrap();
}

#[cfg(test)]
fn body(live: &LivePages) -> String {
    let pages = live.current();
    let page = pages.get("/what-a-sitemap-does/").expect("the post");
    String::from_utf8(page.body().to_vec()).unwrap()
}

#[tokio::test]
async fn an_edit_shows_after_a_refresh() {
    let (folder, renderer) = site("<h1>What a sitemap does</h1>");
    let live = LivePages::new(render_pages(&renderer, None).await.unwrap());
    let app = router(live.clone(), SecurityHeaders::default());
    edit(folder.path(), "<h1>What a sitemap is for</h1>");
    live.refresh(&renderer).await.unwrap();

    assert_eq!(body(&live), "<h1>What a sitemap is for</h1>");
    let request = Request::get("/what-a-sitemap-does/")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let served = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(served.as_ref(), b"<h1>What a sitemap is for</h1>");
}

#[tokio::test]
async fn a_broken_edit_keeps_the_pages_readers_see() {
    let (folder, renderer) = site("<h1>What a sitemap does</h1>");
    let live = LivePages::new(render_pages(&renderer, None).await.unwrap());
    edit(folder.path(), "<h1>{{ broken</h1>");

    let error = live.refresh(&renderer).await.unwrap_err();
    assert!(matches!(error, ServerError::Render { .. }), "{error:?}");
    assert_eq!(body(&live), "<h1>What a sitemap does</h1>");
}

#[tokio::test]
async fn two_pages_at_one_address_keep_the_old_pages() {
    let (folder, renderer) = site("<h1>What a sitemap does</h1>");
    let live = LivePages::new(render_pages(&renderer, None).await.unwrap());
    let twin = under(folder.path(), "what-a-sitemap-does.html");
    fs::write(twin, "<h1>Twin</h1>").unwrap();

    let error = live.refresh(&renderer).await.unwrap_err();
    assert!(
        matches!(error, ServerError::DuplicatePage { .. }),
        "{error:?}"
    );
    assert_eq!(body(&live), "<h1>What a sitemap does</h1>");
}
