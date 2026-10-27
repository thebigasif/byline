//! The newsletter's pages: whole documents with one heading and one
//! sentence, forms that post, nothing a page opens by itself, and every
//! word from the site's language (task 5.5, FRONTEND.md).

use khalid_core::{AbsoluteUrl, Locale};
use khalid_newsletter::{NewsletterSite, Page, Pages, SiteLinks};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[cfg(test)]
fn pages(name: &str, language: &str) -> Pages {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    Pages::new(NewsletterSite {
        name: name.to_owned(),
        locale: Locale::try_new(language).unwrap(),
        links: SiteLinks::new(&address, "/blog/"),
    })
    .unwrap()
}

#[cfg(test)]
fn every_page() -> [Page<'static>; 10] {
    [
        Page::Check,
        Page::WrongEmail,
        Page::Confirm(TOKEN),
        Page::Confirmed,
        Page::Expired,
        Page::Unknown,
        Page::Leave(TOKEN),
        Page::Left,
        Page::Closed,
        Page::Failed,
    ]
}

/// The text of the paragraph right after the heading.
#[cfg(test)]
fn sentence(html: &str) -> &str {
    let start = html.find("</h1>\n<p>").unwrap() + "</h1>\n<p>".len();
    let end = start + html.get(start..).unwrap().find("</p>").unwrap();
    html.get(start..end).unwrap()
}

#[test]
fn every_page_is_a_whole_document_with_one_heading_and_one_sentence() {
    for language in ["en", "de"] {
        let pages = pages("Field Notes", language);
        for page in every_page() {
            let html = pages.html(page);
            assert!(html.starts_with("<!doctype html>\n<html lang=\""), "{html}");
            assert_eq!(html.matches("<h1>").count(), 1, "{page:?}: {html}");
            assert!(html.contains("<meta name=\"robots\" content=\"noindex\">"));
            assert!(html.contains("<a href=\"/blog/\">"), "{page:?}: {html}");
            assert!(!html.contains("<script"), "{page:?}: {html}");
            assert!(!html.contains('{'), "a placeholder was left: {html}");
            let text = sentence(&html);
            assert!(text.ends_with('.'), "{language} {page:?}: {text}");
            assert_eq!(text.matches(". ").count(), 0, "{language} {page:?}: {text}");
        }
    }
}

#[test]
fn a_link_page_asks_for_one_click_that_posts_its_token() {
    let pages = pages("Field Notes", "en");
    let confirm = pages.html(Page::Confirm(TOKEN));
    assert!(confirm.contains("<form method=\"post\" action=\"/blog/subscribe/confirm\">"));
    assert!(confirm.contains(&format!("name=\"token\" value=\"{TOKEN}\"")));
    assert!(confirm.contains("<button type=\"submit\">Yes, send it to me</button>"));
    let leave = pages.html(Page::Leave(TOKEN));
    assert!(leave.contains("<form method=\"post\" action=\"/blog/unsubscribe\">"));
    assert!(
        leave.contains("<h1>Unsubscribe from Field Notes</h1>"),
        "{leave}"
    );
}

#[test]
fn pages_that_need_a_new_sign_up_carry_the_form() {
    let pages = pages("Field Notes", "en");
    for page in [Page::WrongEmail, Page::Expired, Page::Unknown] {
        let html = pages.html(page);
        assert!(html.contains("action=\"/blog/subscribe\""), "{page:?}");
        assert!(html.contains("<label for=\"email\">Your email address</label>"));
        assert!(html.contains("type=\"email\" autocomplete=\"email\" required"));
    }
    assert!(!pages.html(Page::Check).contains("<form"));
}

#[test]
fn the_site_name_is_written_as_text_never_as_markup() {
    let pages = pages("Notes <script>alert(1)</script> & \"more\"", "en");
    let html = pages.html(Page::Left);
    assert!(!html.contains("<script>"), "{html}");
    assert!(html.contains("Notes &lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;more&quot;"));
}

#[test]
fn a_german_site_gets_german_pages_and_others_english() {
    let german = pages("Feldnotizen", "de").html(Page::Confirmed);
    assert!(german.contains("<html lang=\"de\">"), "{german}");
    // "You are subscribed", in German.
    assert!(german.contains("sind angemeldet</h1>"), "{german}");
    let french = pages("Notes", "fr").html(Page::Confirmed);
    assert!(french.contains("<html lang=\"fr\">"), "{french}");
    assert!(french.contains("<h1>You are subscribed</h1>"), "{french}");
}
