//! A site whose own code waits for consent: a reader who has not answered
//! gets the cached page with the banner and no code, a reader who said no
//! gets no code and no banner, and a reader who said yes gets the code
//! with the nonce its policy carries (ADR "consent is a form and a
//! cookie").

use axum::body::{Body, to_bytes};
use axum::http::header::{self, HeaderName};
use axum::http::{Request, Response, StatusCode};
use khalid_server::{
    ASK_END, ASK_START, CHOSEN_END, CHOSEN_START, HELD_END, HELD_START, HeldCode, MediaType,
    NONCE_SLOT, Page, PagePath, Pages, SITE_BODY_SLOT, SITE_HEAD_SLOT, SecurityHeaders,
    router_with_consent,
};
use tower::ServiceExt;

/// A page as the build renders it for a site that asks: the slots, the
/// banner and the hidden form to change a choice, with text long enough
/// to get a Brotli copy.
#[cfg(test)]
fn page_html(head: &str) -> String {
    format!(
        "<html><head><title>Post</title>{head}{SITE_HEAD_SLOT}</head><body>{}{SITE_BODY_SLOT}{ASK_START}<aside>May we count visits?</aside>{ASK_END}{CHOSEN_START}<aside>Change your choice</aside>{CHOSEN_END}</body></html>",
        "<p>A sitemap lists the pages you want found.</p>".repeat(40)
    )
}

#[cfg(test)]
fn pages() -> Pages {
    let path = |path: &str| PagePath::try_new(path).unwrap();
    let held = format!("{HELD_START}<script nonce=\"{NONCE_SLOT}\">post()</script>{HELD_END}");
    let pages = vec![
        Page::new(path("/blog/plain/"), MediaType::Html, page_html("")),
        Page::with_nonce_slot(path("/blog/tagged/"), MediaType::Html, page_html(&held)),
        Page::new(path("/blog/site.css"), MediaType::Css, "p{color:#123}"),
    ];
    Pages::new(pages, page_html("")).unwrap()
}

#[cfg(test)]
async fn get(path: &str, cookie: Option<&str>) -> (Response<Body>, String) {
    let held = HeldCode::new(
        format!("<script nonce=\"{NONCE_SLOT}\">site()</script>"),
        format!("<script nonce=\"{NONCE_SLOT}\">chat()</script>"),
    );
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let request = request.body(Body::empty()).unwrap();
    let router = router_with_consent(pages(), SecurityHeaders::default(), held);
    let response = router.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let text = to_bytes(body, 1 << 20).await.unwrap();
    let text = String::from_utf8_lossy(&text).into_owned();
    (Response::from_parts(parts, Body::empty()), text)
}

#[cfg(test)]
fn header<'r>(response: &'r Response<Body>, name: &HeaderName) -> &'r str {
    response
        .headers()
        .get(name)
        .map_or("", |value| value.to_str().unwrap())
}

#[cfg(test)]
fn directive(response: &Response<Body>, name: &str) -> String {
    let policy = header(response, &header::CONTENT_SECURITY_POLICY);
    let found = policy
        .split(';')
        .map(str::trim)
        .find(|p| p.starts_with(name));
    found
        .unwrap_or_else(|| panic!("no {name} in {policy}"))
        .to_owned()
}

/// The nonce of the first script in `html`.
#[cfg(test)]
fn nonce_in(html: &str) -> String {
    let (_, rest) = html.split_once("<script nonce=\"").unwrap();
    rest.split_once('"').unwrap().0.to_owned()
}

#[tokio::test]
async fn a_reader_who_has_not_answered_gets_the_banner_and_no_code() {
    let (answer, html) = get("/blog/plain/", None).await;
    assert_eq!(answer.status(), StatusCode::OK);
    assert!(html.contains("May we count visits?"), "{html}");
    assert!(!html.contains("()"), "{html}");
    assert!(answer.headers().contains_key(header::ETAG), "the kept page");
    assert_eq!(header(&answer, &header::VARY), "accept-encoding, cookie");
    // A post's own code waits too.
    let (answer, html) = get("/blog/tagged/", None).await;
    assert!(
        html.contains("May we count visits?") && !html.contains("()"),
        "{html}"
    );
    assert_eq!(directive(&answer, "style-src"), "style-src 'self'");
}

#[tokio::test]
async fn a_reader_who_said_yes_gets_every_piece_of_code_with_the_policy_s_nonce() {
    let (answer, html) = get("/blog/tagged/", Some("a=1; khalid-consent=yes")).await;
    assert!(!html.contains("May we count visits?"), "{html}");
    assert!(html.contains("<aside>Change your choice</aside>"), "{html}");
    let nonce = nonce_in(&html);
    assert_eq!(nonce.len(), 32, "{html}");
    for code in ["post()", "site()", "chat()"] {
        let script = format!("<script nonce=\"{nonce}\">{code}</script>");
        assert!(html.contains(&script), "{script} in {html}");
    }
    assert_eq!(
        directive(&answer, "script-src"),
        format!("script-src 'nonce-{nonce}' 'strict-dynamic'")
    );
    assert_eq!(header(&answer, &header::CACHE_CONTROL), "private, no-store");
    assert_eq!(header(&answer, &header::VARY), "cookie");
    assert!(!answer.headers().contains_key(header::ETAG));
}

#[tokio::test]
async fn a_reader_who_said_no_gets_neither_code_nor_banner() {
    let (answer, html) = get("/blog/tagged/", Some("khalid-consent=no")).await;
    assert!(!html.contains("()"), "{html}");
    assert!(!html.contains("May we count visits?"), "{html}");
    assert!(html.contains("<aside>Change your choice</aside>"), "{html}");
    assert_eq!(directive(&answer, "connect-src"), "connect-src 'self'");
    assert_eq!(header(&answer, &header::CACHE_CONTROL), "private, no-store");
}

#[tokio::test]
async fn files_stay_as_they_are_and_the_not_found_page_follows_the_choice() {
    let (answer, body) = get("/blog/site.css", Some("khalid-consent=yes")).await;
    assert_eq!(body, "p{color:#123}");
    assert!(answer.headers().contains_key(header::ETAG));
    let (answer, html) = get("/blog/no-such-post/", Some("khalid-consent=yes")).await;
    assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    assert!(
        html.contains("site()") && !html.contains("May we"),
        "{html}"
    );
}
