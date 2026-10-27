//! The `IndexNow` key file and the payload for changed addresses (KH-SEO-11).

#![cfg(test)]

use khalid_core::{AbsoluteUrl, IndexNowKey, IndexNowSettings, Locale};
use khalid_seo::{IndexNow, SeoError, SeoSite, UrlPath};
use serde_json::{Value, json};

const KEY: &str = "3f9c0b5e2a8d4c71";

fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

/// The demo site: English and German under `/blog/`.
fn demo_site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        url("https://www.example.com"),
        Locale::try_new("en").unwrap(),
    );
    site.mount = UrlPath::try_new("/blog/").unwrap();
    site.languages = vec![
        Locale::try_new("en").unwrap(),
        Locale::try_new("de").unwrap(),
    ];
    site
}

fn settings() -> IndexNowSettings {
    IndexNowSettings::new(IndexNowKey::try_new(KEY).unwrap())
}

fn indexnow() -> IndexNow {
    IndexNow::new(&demo_site(), &settings()).unwrap()
}

fn body(json: &[u8]) -> Value {
    serde_json::from_slice(json).unwrap()
}

#[test]
fn the_key_file_sits_in_the_blog_folder_and_holds_the_key() {
    let file = indexnow().key_file();
    assert_eq!(file.path().as_str(), "/3f9c0b5e2a8d4c71.txt");
    assert_eq!(file.body(), KEY);
    assert_eq!(
        indexnow().key_location().as_str(),
        "https://www.example.com/blog/3f9c0b5e2a8d4c71.txt"
    );
}

#[test]
fn the_payload_names_the_host_the_key_its_file_and_the_changed_addresses() {
    let changed = [
        url("https://www.example.com/blog/why-links-matter/"),
        url("https://www.example.com/blog/de/warum-links-zaehlen/"),
    ];
    let batches = indexnow().submissions(&changed).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(
        body(batches[0].body()),
        json!({
            "host": "www.example.com",
            "key": KEY,
            "keyLocation": "https://www.example.com/blog/3f9c0b5e2a8d4c71.txt",
            "urlList": [
                "https://www.example.com/blog/why-links-matter/",
                "https://www.example.com/blog/de/warum-links-zaehlen/",
            ],
        })
    );
    assert_eq!(batches[0].urls(), &changed);
}

#[test]
fn an_address_changed_twice_is_sent_once_in_first_seen_order() {
    let changed = [
        url("https://www.example.com/blog/b/"),
        url("https://www.example.com/blog/a/"),
        url("https://www.example.com/blog/b/"),
    ];
    let batches = indexnow().submissions(&changed).unwrap();
    let sent: Vec<&str> = batches[0].urls().iter().map(AbsoluteUrl::as_str).collect();
    assert_eq!(
        sent,
        [
            "https://www.example.com/blog/b/",
            "https://www.example.com/blog/a/"
        ]
    );
}

#[test]
fn nothing_changed_means_nothing_to_send() {
    assert_eq!(indexnow().submissions(&[]).unwrap(), Vec::new());
}

#[test]
fn more_than_ten_thousand_addresses_are_split_into_batches() {
    let changed: Vec<AbsoluteUrl> = (0..10_001)
        .map(|n| url(&format!("https://www.example.com/blog/post-{n}/")))
        .collect();
    let batches = indexnow().submissions(&changed).unwrap();
    let sizes: Vec<usize> = batches.iter().map(|batch| batch.urls().len()).collect();
    assert_eq!(sizes, [10_000, 1]);
    assert_eq!(
        body(batches[1].body())["urlList"],
        json!(["https://www.example.com/blog/post-10000/"])
    );
}

#[test]
fn refuses_an_address_outside_the_blog_folder() {
    for outside in [
        "https://www.example.com/pricing/",
        "https://www.example.com/blogroll/",
        "https://shop.example.com/blog/why-links-matter/",
        "http://www.example.com/blog/why-links-matter/",
    ] {
        let changed = [
            url("https://www.example.com/blog/why-links-matter/"),
            url(outside),
        ];
        assert_eq!(
            indexnow().submissions(&changed).unwrap_err(),
            SeoError::IndexNowScope {
                url: outside.to_owned(),
                scope: "https://www.example.com/blog/".to_owned(),
            },
            "{outside}"
        );
    }
}

#[test]
fn sends_to_the_shared_endpoint_unless_settings_name_another() {
    assert_eq!(
        indexnow().endpoint().as_str(),
        "https://api.indexnow.org/indexnow"
    );
    let mut bing = settings();
    bing.endpoint = Some(url("https://www.bing.com/indexnow"));
    let indexnow = IndexNow::new(&demo_site(), &bing).unwrap();
    assert_eq!(
        indexnow.endpoint().as_str(),
        "https://www.bing.com/indexnow"
    );
}

#[test]
fn a_site_at_the_root_keeps_its_key_file_at_the_root() {
    let mut site = demo_site();
    site.mount = UrlPath::root();
    let indexnow = IndexNow::new(&site, &settings()).unwrap();
    assert_eq!(
        indexnow.key_location().as_str(),
        "https://www.example.com/3f9c0b5e2a8d4c71.txt"
    );
    let changed = [url("https://www.example.com/pricing/")];
    assert_eq!(indexnow.submissions(&changed).unwrap().len(), 1);
}
