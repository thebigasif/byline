//! The news sitemap for Google News (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;
mod sitemap_xsd;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{ArticleType, NewsSitemap, PageKind, PublishState, SeoPage};
use sitemap_xsd::{check_urlset, google_extensions};

/// Two fresh news articles, then three that must be left out: one too
/// old, one a draft, one a blog post.
fn news_pages() -> Vec<SeoPage> {
    let mut fresh = internal_links_post();
    fresh.article_type = ArticleType::NewsArticle;
    fresh.published = Some(at(2026, 10, 25, 7));
    let mut fresh_de = internal_links_post_de();
    fresh_de.article_type = ArticleType::NewsArticle;
    fresh_de.published = Some(at(2026, 10, 24, 18));
    let mut old = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path("/core-update-recap/"),
        "Core update recap",
    );
    old.article_type = ArticleType::NewsArticle;
    old.published = Some(at(2026, 10, 22, 7));
    let mut draft = old.clone();
    draft.published = Some(at(2026, 10, 26, 6));
    draft.state = PublishState::Draft;
    let mut blog = internal_links_post();
    blog.published = Some(at(2026, 10, 26, 6));
    vec![fresh, fresh_de, old, draft, blog]
}

fn news_at_noon() -> NewsSitemap {
    NewsSitemap::from_pages(&demo_site(), &news_pages(), at(2026, 10, 26, 12)).unwrap()
}

#[test]
fn the_news_sitemap_lists_news_articles_from_the_last_two_days() {
    insta::assert_snapshot!(news_at_noon().to_xml());
}

#[test]
fn the_news_sitemap_is_valid_against_the_schemas() {
    assert_eq!(
        check_urlset(&news_at_noon().to_xml(), google_extensions),
        Ok(2)
    );
}

#[test]
fn an_article_dated_in_the_future_is_not_news_yet() {
    let news = NewsSitemap::from_pages(&demo_site(), &news_pages(), at(2026, 10, 24, 6)).unwrap();
    assert_eq!(news.entries().len(), 1);
    assert_eq!(news.entries()[0].title, "Core update recap");
}

#[test]
fn chinese_keeps_its_region_in_the_news_language() {
    let mut post = internal_links_post();
    post.article_type = ArticleType::NewsArticle;
    post.published = Some(at(2026, 10, 26, 6));
    post.language = locale("zh-TW");
    let news = NewsSitemap::from_pages(&demo_site(), &[post], at(2026, 10, 26, 12)).unwrap();
    assert!(
        news.to_xml()
            .contains("<news:language>zh-tw</news:language>"),
        "{}",
        news.to_xml()
    );
}
