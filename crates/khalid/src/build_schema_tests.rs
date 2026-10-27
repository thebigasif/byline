//! A post's blocks reach its JSON-LD graph, and the graph has everything
//! Google requires.

use std::path::{Path, PathBuf};

use khalid_content::render;
use khalid_core::{Config, Environment, Locale, SlugMode};
use khalid_seo::{IssueLevel, PageKind, check_graph, page_graph};

use super::fill;
use crate::build_seo;

/// A body with one of each block that writes its own schema, and a link
/// to a Wikipedia article.
const BODY: &str = "\
Read about [search engine optimization](https://en.wikipedia.org/wiki/Search_engine_optimization) first.

::::faq
:::question{text=\"What is a title tag?\"}
The blue link in search results.
:::
::::

::::howto{name=\"Add an internal link\" time=\"10:00\"}
:::step{name=\"Find an older post\"}
Search your site for the topic.
:::
::::

:::review{item=\"Ahrefs Webmaster Tools\" type=\"SoftwareApplication\" rating=\"4.5\" best=\"5\"}
Fast and free for your own site.
:::

::::recipe{name=\"Oat bars\" image=\"oat-bars.jpg\" prep=\"15:00\" cook=\"30:00\"}
:::ingredients
- 250 g rolled oats
:::

:::method
1. Bake at 180 C for 30 minutes.
:::
::::

::video{src=\"/media/site-tour.mp4\" title=\"A tour of the site\" poster=\"site-tour.jpg\" duration=\"2:05\"}
";

/// The demo site's folder.
fn demo() -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.push("../../fixtures/demo-site");
    path
}

#[test]
fn every_block_reaches_the_graph_and_google_finds_nothing_missing() {
    let config = Config::load(&demo(), &Environment::empty()).unwrap();
    let locale = Locale::try_new("en").unwrap();
    let rendered = render(BODY, &locale, SlugMode::Ascii).unwrap();
    let mut seo = build_seo::page(PageKind::Post, &locale, "blocks/", "Blocks").unwrap();
    seo.published = Some(build_seo::midnight(chrono::NaiveDate::default()));
    fill(
        &mut seo,
        &config,
        ("/blog/blocks/", "Asif"),
        (&rendered.structured, &rendered.links),
    )
    .unwrap();

    let site = build_seo::site(&config).unwrap();
    let graph = page_graph(&site, &seo).unwrap();
    let json = graph.to_json();
    for kind in ["FAQPage", "HowTo", "Review", "Recipe", "VideoObject"] {
        assert!(
            json.contains(&format!("\"@type\": \"{kind}\"")),
            "{kind} in {json}"
        );
    }
    assert!(json.contains("\"mentions\""), "{json}");
    assert!(json.contains("https://en.wikipedia.org/wiki/Search_engine_optimization"));
    assert!(
        json.contains("https://www.example.com/blog/blocks/oat-bars.jpg"),
        "{json}"
    );
    assert!(
        json.contains("https://www.example.com/media/site-tour.mp4"),
        "{json}"
    );
    assert!(
        json.contains("\"name\": \"Asif\""),
        "the post's author reviews: {json}"
    );
    assert_eq!(seo.videos.len(), 1, "for the video sitemap");

    let blocking: Vec<_> = check_graph(&graph)
        .into_iter()
        .filter(|issue| issue.level() == IssueLevel::Blocks)
        .map(|issue| format!("{} {}", issue.kind(), issue.property()))
        .collect();
    assert_eq!(blocking, Vec::<String>::new(), "{json}");
}
