//! The cached request path: a post sent from memory, and a 304 for a
//! reader whose copy is current.
//!
//! ```text
//! cargo bench -p khalid-server --bench request
//! ```

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use divan::Bencher;
use khalid_server::{MediaType, Page, PagePath, Pages, router};
use tokio::runtime::Runtime;
use tower::ServiceExt;

/// The post every benchmark asks for.
const POST: &str = "/blog/what-a-sitemap-does/";

fn main() {
    divan::main();
}

/// A site of 1,000 posts of about 30 KB each, the size of a long article.
fn site() -> Option<Router> {
    let body = "<p>A sitemap lists the pages a site wants found.</p>".repeat(600);
    let pages = (0..1_000)
        .map(|n| {
            let path = if n == 0 {
                POST.to_owned()
            } else {
                format!("/blog/post-{n}/")
            };
            Some(Page::new(
                PagePath::try_new(&path).ok()?,
                MediaType::Html,
                body.clone(),
            ))
        })
        .collect::<Option<Vec<Page>>>()?;
    Some(router(Pages::new(pages, "<h1>Page not found</h1>").ok()?))
}

fn run(bencher: Bencher<'_, '_>, tag: Option<&str>) {
    let (Ok(runtime), Some(app)) = (Runtime::new(), site()) else {
        return;
    };
    bencher.bench_local(|| {
        let mut request = Request::get(POST);
        if let Some(tag) = tag {
            request = request.header(header::IF_NONE_MATCH, tag);
        }
        let Ok(request) = request.body(Body::empty()) else {
            return;
        };
        let _ = runtime.block_on(app.clone().oneshot(request));
    });
}

#[divan::bench]
fn cached_post(bencher: Bencher<'_, '_>) {
    run(bencher, None);
}

#[divan::bench]
fn not_modified(bencher: Bencher<'_, '_>) {
    run(bencher, Some("*"));
}
