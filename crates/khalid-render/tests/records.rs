//! What templates see of the input records: identifiers as plain text, modes
//! as lowercase words, absent values as nothing.

use khalid_core::{AbsoluteUrl, Locale};
use khalid_render::{
    Alternate, Features, Nav, NavLink, PageKind, PageMeta, RenderSite, Robots, ShareImage,
};
use serde_json::json;

#[test]
fn page_meta_reaches_templates_as_plain_text() {
    let meta = PageMeta {
        title: "What a sitemap does".to_owned(),
        description: "A sitemap lists the pages you want found.".to_owned(),
        canonical: AbsoluteUrl::parse("https://www.example.com/blog/what-a-sitemap-does/").unwrap(),
        path: "/blog/what-a-sitemap-does/".to_owned(),
        kind: PageKind::Article,
        robots: Robots::NoIndex,
        image: Some(ShareImage {
            url: AbsoluteUrl::parse("https://www.example.com/blog/sitemap.png").unwrap(),
            alt: "A sitemap drawn as a tree".to_owned(),
            width: 1200,
            height: 630,
        }),
        alternates: vec![Alternate {
            locale: Locale::try_new("de").unwrap(),
            href: AbsoluteUrl::parse("https://www.example.com/blog/de/was-eine-sitemap-leistet/")
                .unwrap(),
            label: "Deutsch".to_owned(),
        }],
        x_default: None,
        seo_head: None,
    };
    let seen = serde_json::to_value(&meta).unwrap();
    assert_eq!(
        seen["canonical"],
        "https://www.example.com/blog/what-a-sitemap-does/"
    );
    assert_eq!(seen["kind"], "article");
    assert_eq!(seen["robots"], "noindex");
    assert_eq!(
        seen["image"]["url"],
        "https://www.example.com/blog/sitemap.png"
    );
    assert_eq!(seen["image"]["width"], 1200);
    assert_eq!(seen["alternates"][0]["locale"], "de");
    assert_eq!(seen["x_default"], json!(null));
}

#[test]
fn the_site_hides_its_locale_from_templates() {
    let site = RenderSite {
        name: "Field Notes on SEO".to_owned(),
        locale: Locale::try_new("en").unwrap(),
        home: "/blog/".to_owned(),
        stylesheet: "/blog/assets/site.0123abcd.css".to_owned(),
        nav: Nav {
            primary: vec![NavLink {
                label: "Glossary".to_owned(),
                href: "/blog/glossary/".to_owned(),
            }],
            footer: Vec::new(),
        },
        main_site: None,
        share_image: None,
        features: Features::all(),
    };
    let seen = serde_json::to_value(&site).unwrap();
    assert!(
        seen.get("locale").is_none(),
        "templates read lang, not site.locale"
    );
    assert_eq!(seen["nav"]["primary"][0]["href"], "/blog/glossary/");
    assert_eq!(seen["nav"]["footer"], json!([]));
}
