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
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tokio::runtime::Runtime;
use tower::ServiceExt;

/// The post every benchmark asks for.
const POST: &str = "/blog/what-a-sitemap-does/";

fn main() {
    divan::main();
}

/// The words the made-up posts are written with.
const WORDS: [&str; 24] = [
    "sitemap", "search", "engines", "find", "pages", "links", "the", "a", "site", "wants", "every",
    "post", "lists", "internal", "anchor", "text", "crawl", "index", "fresh", "date", "author",
    "reader", "title", "heading",
];

/// About 30 KB of HTML, the size of a long article, with words in a
/// seeded order so it compresses like prose, not like one repeated line.
fn article(seed: u64) -> String {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let mut html = String::with_capacity(31_000);
    while html.len() < 30_000 {
        html.push_str("<p>");
        for _ in 0..14 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let index = usize::try_from((state >> 33) % 24).unwrap_or(0);
            html.push_str(WORDS.get(index).copied().unwrap_or("post"));
            html.push(' ');
        }
        html.push_str("</p>\n");
    }
    html
}

/// A post at `path` with a seeded article.
fn post(path: &str, seed: u64) -> Option<Page> {
    Some(Page::new(
        PagePath::try_new(path).ok()?,
        MediaType::Html,
        article(seed),
    ))
}

/// A site of 1,000 posts.
fn site() -> Option<Router> {
    let pages = (0..1_000_u64)
        .map(|n| match n {
            0 => post(POST, n),
            _ => post(&format!("/blog/post-{n}/"), n),
        })
        .collect::<Option<Vec<Page>>>()?;
    let pages = Pages::new(pages, "<h1>Page not found</h1>").ok()?;
    Some(router(pages, SecurityHeaders::default()))
}

/// Building the set for one post: its tag and its Brotli copy.
#[divan::bench]
fn build_one_post(bencher: Bencher<'_, '_>) {
    bencher
        .with_inputs(|| post(POST, 7))
        .bench_values(|page| page.map(|page| Pages::new(vec![page], "")));
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
