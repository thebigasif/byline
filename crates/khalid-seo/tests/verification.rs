//! Site verification tags and files (task 1.13, KH-SEO-7).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale};
use khalid_seo::{
    PageKind, RootFile, SeoError, SeoPage, SiteVerification, UrlPath, VerificationCode, head_tags,
};

fn code(text: &str) -> VerificationCode {
    VerificationCode::try_new(text).unwrap()
}

fn verified_site() -> khalid_seo::SeoSite {
    let mut site = demo_site();
    let mut verification = SiteVerification::default();
    verification.google = Some(code("dBw5CvburAxi537Rp9qi5uG2174Vb6JwHwIRwPSLIK8"));
    verification.bing = Some(code("1234ABCD5678EF90AB12CD34EF56AB78"));
    verification.yandex = Some(code("4a1b2c3d4e5f6a7b"));
    verification.baidu = Some(code("codeva-Xy7Q2wE9rT"));
    verification.pinterest = Some(code("a1b2c3d4e5f60718293a4b5c6d7e8f90"));
    site.verification = verification;
    site
}

#[test]
fn the_home_page_carries_every_verification_tag_in_a_fixed_order() {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let html = head_tags(&verified_site(), &home).unwrap().to_html();
    let verification: Vec<&str> = html
        .lines()
        .filter(|line| line.contains("verif") || line.contains("msvalidate"))
        .collect();
    insta::assert_snapshot!(verification.join("\n"));
}

#[test]
fn other_pages_carry_none() {
    for post in [internal_links_post(), internal_links_post_de()] {
        let html = head_tags(&verified_site(), &post).unwrap().to_html();
        assert!(!html.contains("verification"), "{html}");
        assert!(!html.contains("msvalidate"), "{html}");
    }
}

#[test]
fn a_code_with_markup_or_spaces_is_refused() {
    for bad in ["", "abc def", r#"abc"><script>"#, "abc\n"] {
        assert!(
            matches!(
                VerificationCode::try_new(bad),
                Err(SeoError::VerificationCode { .. })
            ),
            "{bad:?}"
        );
    }
}

#[test]
fn a_verification_file_lives_at_the_root_with_a_plain_name() {
    let file = RootFile::try_new(
        "google1a2b3c4d5e6f7a8b.html",
        "google-site-verification: google1a2b3c4d5e6f7a8b.html",
    )
    .unwrap();
    assert_eq!(file.path().as_str(), "/google1a2b3c4d5e6f7a8b.html");
    assert_eq!(
        file.body(),
        "google-site-verification: google1a2b3c4d5e6f7a8b.html"
    );
    for bad in [
        "",
        "../secret.txt",
        "a/b.html",
        ".env",
        "robots.txt",
        "a b.html",
    ] {
        assert!(
            matches!(RootFile::try_new(bad, "x"), Err(SeoError::RootFile { .. })),
            "{bad:?}"
        );
    }
}
