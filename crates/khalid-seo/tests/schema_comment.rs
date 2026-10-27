//! A post's approved comments in its article schema (task 5.4, KH-SEO-3).

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{SchemaGraph, SeoComment, page_graph};
use serde_json::{Value, json};

fn comment(id: i64, author: &str, text: &str, replies: Vec<SeoComment>) -> SeoComment {
    SeoComment {
        id,
        author: author.to_owned(),
        text: text.to_owned(),
        published: at(2026, 10, 27, 9),
        replies,
    }
}

fn article(post: &khalid_seo::SeoPage) -> Value {
    let graph = page_graph(&demo_site(), post).unwrap();
    let article = SchemaGraph::new(vec![graph.nodes()[4].clone()]);
    let json: Value = serde_json::from_str(&article.to_json()).unwrap();
    json["@graph"][0].clone()
}

#[test]
fn approved_comments_and_their_replies_are_listed_with_a_count() {
    let mut post = internal_links_post();
    let reply = comment(2, "Asif", "Thanks, Lena.", vec![]);
    post.comments = vec![
        comment(1, "Lena", "Great <post>.", vec![reply]),
        comment(3, "Ben", "Useful.", vec![]),
    ];
    let article = article(&post);
    assert_eq!(article["commentCount"], 3);
    let page = "https://www.example.com/blog/why-internal-links-matter/";
    let first = &article["comment"][0];
    assert_eq!(
        first,
        &json!({
            "@type": "Comment",
            "@id": format!("{page}#comment-1"),
            "url": format!("{page}#comment-1"),
            "author": {"@type": "Person", "name": "Lena"},
            "text": "Great <post>.",
            "datePublished": "2026-10-27T09:00:00Z",
            "comment": [{
                "@type": "Comment",
                "@id": format!("{page}#comment-2"),
                "url": format!("{page}#comment-2"),
                "author": {"@type": "Person", "name": "Asif"},
                "text": "Thanks, Lena.",
                "datePublished": "2026-10-27T09:00:00Z"
            }]
        })
    );
    assert_eq!(article["comment"][1]["@id"], format!("{page}#comment-3"));
}

#[test]
fn a_post_without_comments_says_nothing_about_them() {
    for post in [internal_links_post(), internal_links_post_de()] {
        let article = article(&post);
        assert!(article.get("comment").is_none());
        assert!(article.get("commentCount").is_none());
    }
}
