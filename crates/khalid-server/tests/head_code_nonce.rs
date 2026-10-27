//! A page with a post's head code gets its request's nonce in the page and
//! in the policy, and is never kept (ADR "head code gets the nonce on each
//! request").

use axum::body::{Body, to_bytes};
use axum::http::header::{self, HeaderName};
use axum::http::{Request, Response};
use khalid_server::{MediaType, NONCE_SLOT, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

#[cfg(test)]
async fn get(path: &str) -> Response<Body> {
    let head = format!(
        "<script nonce=\"{NONCE_SLOT}\">go()</script>{}",
        "<p>Text.</p>".repeat(50)
    );
    let pages = vec![
        Page::with_nonce_slot(
            PagePath::try_new("/blog/tagged/").unwrap(),
            MediaType::Html,
            head,
        ),
        Page::new(
            PagePath::try_new("/blog/plain/").unwrap(),
            MediaType::Html,
            "<p>Plain.</p>",
        ),
    ];
    let pages = Pages::new(pages, "<h1>Page not found</h1>").unwrap();
    let request = Request::get(path)
        .header(header::ACCEPT_ENCODING, "br")
        .body(Body::empty())
        .unwrap();
    router(pages, SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap()
}

#[cfg(test)]
fn header<'r>(response: &'r Response<Body>, name: &HeaderName) -> Option<&'r str> {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().unwrap())
}

#[cfg(test)]
fn directive(policy: &str, name: &str) -> String {
    let found = policy
        .split(';')
        .map(str::trim)
        .find(|part| part.starts_with(name));
    found
        .unwrap_or_else(|| panic!("no {name} in {policy}"))
        .to_owned()
}

#[tokio::test]
async fn the_page_and_its_policy_carry_the_same_fresh_nonce() {
    let first = get("/blog/tagged/").await;
    let policy = header(&first, &header::CONTENT_SECURITY_POLICY)
        .unwrap()
        .to_owned();
    assert_eq!(header(&first, &header::CACHE_CONTROL), Some("no-store"));
    assert_eq!(header(&first, &header::ETAG), None);
    assert_eq!(header(&first, &header::CONTENT_ENCODING), None);
    let body = to_bytes(first.into_body(), 1 << 20).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body.contains(NONCE_SLOT), "{body}");
    let nonce = body
        .strip_prefix("<script nonce=\"")
        .and_then(|rest| rest.split_once('"'))
        .map(|(nonce, _)| nonce.to_owned())
        .unwrap();
    assert_eq!(nonce.len(), 32);
    assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()), "{nonce}");
    let script = format!("script-src 'nonce-{nonce}' 'strict-dynamic'");
    assert_eq!(directive(&policy, "script-src"), script);
    assert_eq!(
        directive(&policy, "style-src"),
        format!("style-src 'self' 'nonce-{nonce}'")
    );
    assert_eq!(
        directive(&policy, "connect-src"),
        "connect-src 'self' https:"
    );
    let second = get("/blog/tagged/").await;
    let again = header(&second, &header::CONTENT_SECURITY_POLICY).unwrap();
    assert!(!again.contains(&nonce), "each request has its own nonce");
}

#[tokio::test]
async fn other_pages_keep_the_strict_policy_and_their_tag() {
    let plain = get("/blog/plain/").await;
    let policy = header(&plain, &header::CONTENT_SECURITY_POLICY).unwrap();
    assert_eq!(directive(policy, "style-src"), "style-src 'self'");
    assert_eq!(directive(policy, "connect-src"), "connect-src 'self'");
    assert!(header(&plain, &header::ETAG).is_some());
}
