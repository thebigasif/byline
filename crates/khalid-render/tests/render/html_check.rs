//! The checks every rendered page must pass, without a browser.

use scraper::{ElementRef, Html, Selector};

fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

/// Checks what a machine can check of FRONTEND.md and RULES.md BL-WEB-1,
/// BL-WEB-2 and BL-WEB-5 without a browser: the HTML parses without a single
/// error, there is one `h1` and headings do not skip levels, `lang` and `dir`
/// are set, landmarks are labelled, the skip link lands, ids are unique,
/// images reserve their space, every link and form goes somewhere, and there
/// is no script and no inline style.
pub(crate) fn assert_valid_page(page: &str) {
    let html = Html::parse_document(page);
    assert!(
        html.errors.is_empty(),
        "HTML parse errors: {:?}",
        html.errors
    );
    assert!(page.starts_with("<!doctype html>"), "a doctype first");
    assert_document(&html);
    assert_headings(&html);
    assert_landmarks(&html);
    assert_unique_ids(&html);
    assert_elements(&html);
    assert!(
        !page.contains(" style="),
        "no inline style, so the CSP needs no exception"
    );
}

fn assert_document(html: &Html) {
    let root = select(html, "html");
    let root = root.first().unwrap().value();
    assert!(
        root.attr("lang").is_some_and(|v| !v.is_empty()),
        "html needs lang"
    );
    assert!(
        matches!(root.attr("dir"), Some("ltr" | "rtl")),
        "html needs dir"
    );
}

fn assert_headings(html: &Html) {
    assert_eq!(select(html, "h1").len(), 1, "exactly one h1");
    let mut previous = 1;
    for heading in select(html, "h1, h2, h3, h4, h5, h6") {
        let name = heading.value().name();
        let level: u32 = name.strip_prefix('h').unwrap().parse().unwrap();
        assert!(
            level <= previous + 1,
            "h{level} skips a level after h{previous}"
        );
        previous = level;
    }
}

fn assert_landmarks(html: &Html) {
    assert_eq!(
        select(html, "main#main").len(),
        1,
        "one main, the skip link's target"
    );
    assert_eq!(
        select(html, "a.skip-link[href='#main']").len(),
        1,
        "a skip link to main"
    );
    assert_eq!(select(html, "body > header").len(), 1, "one banner header");
    assert_eq!(select(html, "body > footer").len(), 1, "one content footer");
    for nav in select(html, "nav") {
        assert!(
            nav.value().attr("aria-label").is_some(),
            "every nav is labelled"
        );
    }
}

fn assert_unique_ids(html: &Html) {
    let mut ids: Vec<&str> = select(html, "[id]")
        .iter()
        .filter_map(|element| element.value().id())
        .collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "ids are unique");
}

fn assert_elements(html: &Html) {
    for image in select(html, "img") {
        for attribute in ["alt", "width", "height", "srcset", "sizes"] {
            assert!(
                image.value().attr(attribute).is_some(),
                "img needs {attribute}"
            );
        }
    }
    for link in select(html, "a") {
        let href = link.value().attr("href").unwrap_or_default();
        assert!(
            !href.is_empty() && href != "#",
            "a link that goes nowhere: {href:?}"
        );
    }
    for form in select(html, "form") {
        assert!(
            form.value().attr("action").is_some(),
            "every form has an action"
        );
    }
    assert!(
        select(html, "script").is_empty(),
        "no script on a public page"
    );
}

/// A page that passes every check, for the tests that break one.
const GOOD: &str = r##"<!doctype html><html lang="en" dir="ltr"><head><title>Hi</title></head><body><a class="skip-link" href="#main">Skip</a><header></header><main id="main"><h1>Hi</h1></main><footer></footer></body></html>"##;

#[test]
fn the_checker_accepts_a_good_page() {
    assert_valid_page(GOOD);
}

#[test]
#[should_panic(expected = "exactly one h1")]
fn the_checker_refuses_a_second_h1() {
    assert_valid_page(&GOOD.replace("<h1>Hi</h1>", "<h1>Hi</h1><h1>Again</h1>"));
}

#[test]
#[should_panic(expected = "HTML parse errors")]
fn the_checker_refuses_broken_html() {
    assert_valid_page(&GOOD.replace("<h1>Hi</h1>", "<h1>Hi</h2>"));
}
