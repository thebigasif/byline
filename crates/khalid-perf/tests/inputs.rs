//! A generated site turns into pages that render and posts that lint.

use khalid_core::SystemClock;
use khalid_lint::{LintConfig, Linter};
use khalid_perf::{Fixture, PrepareError, Prepared};
use khalid_render::Renderer;
use tempfile::TempDir;

#[cfg(test)]
fn prepared(posts: usize) -> (TempDir, Prepared) {
    let dir = tempfile::tempdir().unwrap();
    Fixture::new(posts).unwrap().write(dir.path()).unwrap();
    let prepared = Prepared::read(dir.path()).unwrap();
    (dir, prepared)
}

#[test]
fn every_post_becomes_a_page_that_renders_with_its_title() {
    let (_dir, site) = prepared(30);
    assert_eq!(site.pages.len(), 30);
    let renderer = Renderer::new().unwrap();
    for page in &site.pages {
        let html = renderer.render(&site.site, &page.meta, &page.body).unwrap();
        assert_eq!(html.matches("<h1").count(), 1, "{}", page.meta.path);
        assert!(html.contains(&page.meta.title), "{}", page.meta.path);
        assert!(html.contains("<h2"), "{}", page.meta.path);
    }
}

#[test]
fn every_post_lints_without_a_block_and_with_its_links() {
    let (_dir, site) = prepared(30);
    assert_eq!(site.lint.posts.len(), 30);
    for post in &site.lint.posts {
        assert_eq!(post.links.len(), 4, "{}", post.slug);
        assert!(post.word_count >= 300, "{}", post.slug);
        assert!(!post.body_text.contains('<'), "{}", post.slug);
    }
    let linter = Linter::new(&LintConfig::default(), &SystemClock).unwrap();
    assert!(!linter.lint_site(&site.lint).blocks());
}

#[test]
fn a_folder_without_settings_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let result = Prepared::read(dir.path());
    assert!(matches!(result, Err(PrepareError::Config(_))), "{result:?}");
}
