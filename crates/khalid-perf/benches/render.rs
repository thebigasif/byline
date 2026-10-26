//! Post page render, in process, over the 1,000-post site (budget: p50
//! under 1 ms, p99 under 5 ms). Each sample renders the next post, so the
//! numbers cover the whole site rather than one page.
//!
//! ```text
//! cargo bench -p khalid-perf --bench render
//! ```

use std::sync::LazyLock;

use divan::Bencher;
use khalid_perf::{Fixture, Prepared};
use khalid_render::Renderer;

fn main() {
    divan::main();
}

/// The 1,000-post site, written and read once for every benchmark.
static SITE: LazyLock<Option<Prepared>> = LazyLock::new(|| {
    let dir = tempfile::tempdir().ok()?;
    Fixture::THOUSAND.write(dir.path()).ok()?;
    Prepared::read(dir.path()).ok()
});

/// One post page, the next one each time.
#[divan::bench]
fn post_page(bencher: Bencher<'_, '_>) {
    let (Some(site), Ok(renderer)) = (SITE.as_ref(), Renderer::new()) else {
        return;
    };
    let mut pages = site.pages.iter().cycle();
    bencher.bench_local(|| {
        let page = pages.next()?;
        renderer
            .render(&site.site, &page.meta, divan::black_box(&page.body))
            .ok()
    });
}
