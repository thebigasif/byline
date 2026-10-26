//! Robots directives per page and the index policy per page type (task 1.13, KH-SEO-9).

use khalid_core::{
    Following, ImagePreview, Indexing, Locale, PreviewLimit, Restriction, RobotsDirectives,
};
use khalid_seo::{IndexPolicy, PageKind, SeoPage, UrlPath};

#[cfg(test)]
fn page(kind: PageKind, posts: Option<u32>) -> SeoPage {
    let mut page = SeoPage::new(
        kind,
        Locale::try_new("en").unwrap(),
        UrlPath::try_new("/tag/crawl-budget/").unwrap(),
        "Crawl budget",
    );
    page.item_count = posts;
    page
}

#[cfg(test)]
fn robots(kind: PageKind, posts: Option<u32>) -> String {
    IndexPolicy::default()
        .directives_for(&page(kind, posts))
        .to_string()
}

#[test]
fn a_post_may_be_indexed_with_full_snippets_and_large_images() {
    assert_eq!(
        robots(PageKind::Post, None),
        "index, follow, max-snippet:-1, max-image-preview:large, max-video-preview:-1"
    );
}

#[test]
fn search_results_date_archives_and_not_found_are_noindex() {
    for kind in [PageKind::Search, PageKind::Date, PageKind::NotFound] {
        assert_eq!(robots(kind, None), "noindex, follow", "{kind:?}");
    }
}

#[test]
fn a_thin_tag_or_author_archive_is_noindex_until_it_has_three_posts() {
    for kind in [PageKind::Tag, PageKind::Author] {
        assert_eq!(robots(kind, Some(2)), "noindex, follow", "{kind:?}");
        assert!(
            robots(kind, Some(3)).starts_with("index, follow"),
            "{kind:?}"
        );
    }
    assert!(robots(PageKind::Category, Some(1)).starts_with("index, follow"));
}

#[test]
fn a_site_can_change_the_setting_for_one_page_type() {
    let mut policy = IndexPolicy::default();
    policy.by_kind.insert(PageKind::Category, Indexing::NoIndex);
    policy.thin_archive_below = 0;
    let category = policy.directives_for(&page(PageKind::Category, Some(10)));
    assert_eq!(category.indexing, Indexing::NoIndex);
    let tag = policy.directives_for(&page(PageKind::Tag, Some(1)));
    assert_eq!(tag.indexing, Indexing::Index);
}

#[test]
fn a_page_s_own_directives_replace_the_policy_whole() {
    let mut own = RobotsDirectives::default();
    own.following = Following::NoFollow;
    own.restrictions = [Restriction::NoSnippet, Restriction::NoArchive].into();
    own.max_image_preview = ImagePreview::None;
    own.max_video_preview = PreviewLimit::AtMost(30);
    let mut search = page(PageKind::Search, None);
    search.robots = Some(own);
    assert_eq!(
        IndexPolicy::default().directives_for(&search).to_string(),
        "index, nofollow, noarchive, nosnippet, max-snippet:-1, \
         max-image-preview:none, max-video-preview:30"
    );
}
