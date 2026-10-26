//! The site-wide link policy (task 1.13, KH-SEO-10).

#![cfg(test)]

mod common;

use std::fmt::Write as _;

use common::{demo_site, internal_links_post, internal_links_post_de};
use khalid_seo::{Domain, LinkRel, LinkTarget, SeoError, SeoSite};

fn domain(text: &str) -> Domain {
    Domain::try_new(text).unwrap()
}

/// Every external link `nofollow` and in a new tab, affiliate links
/// `sponsored`, and the app at `app.serpdad.com` counted as the same site.
fn demo_policy_site() -> SeoSite {
    let mut site = demo_site();
    site.links.external_rel = [LinkRel::NoFollow].into();
    site.links.external_target = LinkTarget::NewTab;
    site.links.affiliate_domains = vec![domain("amazon.com"), domain("partner.example.org")];
    site.links.internal_domains = vec![domain("app.serpdad.com")];
    site
}

fn attributes(site: &SeoSite, href: &str) -> String {
    let link = site.links.attributes_for(site, href);
    format!(
        "{href} -> rel={} target={}",
        link.rel().unwrap_or_else(|| "-".to_owned()),
        link.target().unwrap_or("-")
    )
}

#[test]
fn each_kind_of_link_gets_the_policy_attributes() {
    let site = demo_policy_site();
    let mut table = String::new();
    for href in [
        "/blog/why-internal-links-matter/",
        "#anchor-text",
        "https://www.example.com/pricing/",
        "https://example.com/about/",
        "https://app.serpdad.com/briefs/",
        "https://developers.google.com/search/",
        "//developers.google.com/search/",
        "https://www.amazon.com/dp/B0C1234567",
        "https://partner.example.org/offer",
        "https://notpartner.example.org/offer",
        "mailto:hello@example.com",
    ] {
        writeln!(table, "{}", attributes(&site, href)).unwrap();
    }
    insta::assert_snapshot!(table);
}

#[test]
fn an_affiliate_link_is_sponsored_even_when_externals_are_followed() {
    let mut site = demo_site();
    site.links.affiliate_domains = vec![domain("amazon.com")];
    let link = site
        .links
        .attributes_for(&site, "https://amazon.com/dp/B0C1234567");
    assert_eq!(link.rel().as_deref(), Some("sponsored"));
    let plain = site
        .links
        .attributes_for(&site, "https://developers.google.com/");
    assert_eq!(plain.rel(), None);
    assert_eq!(plain.target(), None);
}

#[test]
fn the_default_policy_leaves_every_link_alone() {
    let site = demo_site();
    for post in [internal_links_post(), internal_links_post_de()] {
        let address = site.url_for(&post.language, &post.path).unwrap();
        let link = site.links.attributes_for(&site, address.as_str());
        assert_eq!((link.rel(), link.target()), (None, None));
    }
    let external = site
        .links
        .attributes_for(&site, "https://developers.google.com/");
    assert_eq!((external.rel(), external.target()), (None, None));
}

#[test]
fn a_domain_is_checked() {
    assert_eq!(domain("Amazon.COM").as_str(), "amazon.com");
    for bad in [
        "",
        "amazon com",
        "https://amazon.com",
        "amazon.com/dp",
        ".amazon.com",
    ] {
        assert!(
            matches!(Domain::try_new(bad), Err(SeoError::Domain { .. })),
            "{bad:?}"
        );
    }
}
