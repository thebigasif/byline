//! Linting over the 1,000-post site: one post against the whole site
//! (budget: under 20 ms), and every rule over the whole site.
//!
//! ```text
//! cargo bench -p khalid-perf --bench lint
//! ```

use std::sync::LazyLock;

use divan::Bencher;
use khalid_core::SystemClock;
use khalid_lint::{LintConfig, Linter};
use khalid_perf::{Fixture, Prepared};

fn main() {
    divan::main();
}

/// The 1,000-post site, written and read once for every benchmark.
static SITE: LazyLock<Option<Prepared>> = LazyLock::new(|| {
    let dir = tempfile::tempdir().ok()?;
    Fixture::THOUSAND.write(dir.path()).ok()?;
    Prepared::read(dir.path()).ok()
});

/// One post, the next one each time, as `khalid publish` lints it.
#[divan::bench]
fn one_post(bencher: Bencher<'_, '_>) {
    let (Some(site), Ok(linter)) = (
        SITE.as_ref(),
        Linter::new(&LintConfig::default(), &SystemClock),
    ) else {
        return;
    };
    let mut posts = site.lint.posts.iter().cycle();
    bencher.bench_local(|| {
        let post = posts.next()?;
        Some(linter.lint_post(divan::black_box(post), &site.lint))
    });
}

/// Every rule over all 1,000 posts, as `khalid lint` runs it.
#[divan::bench(sample_count = 3, sample_size = 1)]
fn whole_site(bencher: Bencher<'_, '_>) {
    let (Some(site), Ok(linter)) = (
        SITE.as_ref(),
        Linter::new(&LintConfig::default(), &SystemClock),
    ) else {
        return;
    };
    bencher.bench_local(|| linter.lint_site(divan::black_box(&site.lint)));
}
