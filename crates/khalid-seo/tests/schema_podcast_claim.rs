//! The podcast and fact-check blocks: `PodcastEpisode` and `ClaimReview`
//! (brief G15, KH-SEO-3). Google shows no rich result for either, so the
//! check raises nothing.

#![cfg(test)]

mod common;

use std::time::Duration;

use common::{at, demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    ClaimReview, PodcastEpisode, Rating, SchemaBlock, SchemaGraph, SeoPage, check_graph, page_graph,
};
use serde_json::{Value, json};

const PAGE: &str = "https://www.example.com/blog/why-internal-links-matter/";

/// The graph of `post`, published at noon on 2026-10-01, with `block`,
/// and its last node as JSON.
fn with_block(mut post: SeoPage, block: SchemaBlock) -> (SchemaGraph, Value) {
    post.published = Some(at(2026, 10, 1, 12));
    post.blocks = vec![block];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let last = graph.nodes().last().unwrap().clone();
    let mut json: Value = serde_json::from_str(&SchemaGraph::new(vec![last]).to_json()).unwrap();
    (graph, json["@graph"][0].take())
}

fn issues_about(graph: &SchemaGraph, kind: &str) -> Vec<String> {
    let issues = check_graph(graph).into_iter();
    let found = issues.filter(|issue| issue.kind() == kind);
    found.map(|issue| issue.property().to_owned()).collect()
}

#[test]
fn an_episode_belongs_to_its_show_and_carries_its_audio() {
    let mut episode = PodcastEpisode::new("Why orphan pages hurt", "Field Notes on SEO");
    episode.series_url = Some(url("https://www.example.com/blog/podcast/"));
    episode.number = Some(12);
    episode.audio = Some(url("https://www.example.com/blog/media/episode-12.mp3"));
    episode.duration = Some(Duration::from_secs(1520));
    let (graph, node) = with_block(internal_links_post(), SchemaBlock::Podcast(episode));
    let want = json!({
        "@type": "PodcastEpisode",
        "@id": format!("{PAGE}#episode-1"),
        "name": "Why orphan pages hurt",
        "partOfSeries": {
            "@type": "PodcastSeries",
            "name": "Field Notes on SEO",
            "url": "https://www.example.com/blog/podcast/"
        },
        "episodeNumber": 12,
        "datePublished": "2026-10-01T12:00:00Z",
        "associatedMedia": {
            "@type": "AudioObject",
            "contentUrl": "https://www.example.com/blog/media/episode-12.mp3",
            "duration": "PT25M20S"
        }
    });
    assert_eq!(node, want);
    assert_eq!(issues_about(&graph, "PodcastEpisode"), Vec::<String>::new());
}

#[test]
fn a_fact_check_gives_its_verdict_in_words_and_as_a_score() {
    let check = ClaimReview::new(
        "Internal links pass no ranking value.",
        Rating::new(1.0, 5.0).unwrap(),
        "False",
    );
    let (graph, node) = with_block(internal_links_post_de(), SchemaBlock::FactCheck(check));
    let page = "https://www.example.com/blog/de/warum-interne-links-wichtig-sind/";
    let want = json!({
        "@type": "ClaimReview",
        "@id": format!("{page}#claim-1"),
        "url": page,
        "claimReviewed": "Internal links pass no ranking value.",
        "reviewRating": {
            "@type": "Rating",
            "ratingValue": 1.0, "bestRating": 5.0, "worstRating": 0,
            "alternateName": "False"
        },
        "author": {"@id": "https://www.example.com/blog/#organization"},
        "datePublished": "2026-10-01T12:00:00Z"
    });
    assert_eq!(node, want);
    assert_eq!(issues_about(&graph, "ClaimReview"), Vec::<String>::new());
    assert_eq!(issues_about(&graph, "Rating"), Vec::<String>::new());
}
