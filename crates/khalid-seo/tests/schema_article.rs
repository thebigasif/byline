//! Article schema (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{ArticleType, SchemaGraph, SchemaValue, ShareImage, page_graph};

#[test]
fn a_post_is_a_blog_posting() {
    let mut post = internal_links_post();
    post.word_count = Some(1240);
    post.share_image = Some(ShareImage::new(
        url("https://www.example.com/blog/media/internal-links.png"),
        1200,
        630,
        "A site map with arrows between pages",
    ));
    post.images = vec![url(
        "https://www.example.com/blog/media/internal-links-map.png",
    )];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let article = SchemaGraph::new(graph.nodes()[4..].to_vec());
    insta::assert_snapshot!(article.to_json());
}

#[test]
fn the_content_type_picks_the_article_type() {
    for (article_type, kind) in [
        (ArticleType::Article, "Article"),
        (ArticleType::BlogPosting, "BlogPosting"),
        (ArticleType::NewsArticle, "NewsArticle"),
        (ArticleType::TechArticle, "TechArticle"),
    ] {
        let mut post = internal_links_post();
        post.article_type = article_type;
        let graph = page_graph(&demo_site(), &post).unwrap();
        assert_eq!(graph.nodes()[4].kind(), kind);
    }
}

#[test]
fn a_post_never_changed_is_dated_modified_when_published() {
    let mut post = internal_links_post_de();
    post.published = internal_links_post().published;
    let graph = page_graph(&demo_site(), &post).unwrap();
    let article = &graph.nodes()[4];
    assert_eq!(article.get("dateModified"), article.get("datePublished"));
    assert!(article.get("datePublished").is_some());
}

#[test]
fn a_post_without_an_author_names_none() {
    let mut post = internal_links_post();
    post.author = None;
    let graph = page_graph(&demo_site(), &post).unwrap();
    assert_eq!(graph.nodes()[4].get("author"), None);
}

#[test]
fn a_long_title_is_cut_to_110_characters_for_the_headline() {
    let mut post = internal_links_post();
    post.title = "Internal links ".repeat(10);
    let graph = page_graph(&demo_site(), &post).unwrap();
    let Some(SchemaValue::Text(headline)) = graph.nodes()[4].get("headline") else {
        panic!("no headline");
    };
    assert_eq!(headline.chars().count(), 110);
}
