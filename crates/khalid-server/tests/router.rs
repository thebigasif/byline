//! The router serves each page at its clean address and nothing else.

use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use khalid_server::{MediaType, Moved, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

#[cfg(test)]
fn page(path: &str, media_type: MediaType, body: &'static str) -> Page {
    Page::new(PagePath::try_new(path).unwrap(), media_type, body)
}

#[cfg(test)]
fn demo() -> Pages {
    Pages::new(
        vec![
            page("/blog/", MediaType::Html, "<h1>The blog</h1>"),
            page(
                "/blog/what-a-sitemap-does/",
                MediaType::Html,
                "<h1>What a sitemap does</h1>",
            ),
            page("/de/blog/über-seo/", MediaType::Html, "<h1>Über SEO</h1>"),
            page("/robots.txt", MediaType::PlainText, "User-agent: *\n"),
            page("/sitemap.xml", MediaType::Xml, "<urlset/>"),
        ],
        "<h1>Page not found</h1>",
    )
    .unwrap()
}

#[cfg(test)]
async fn send(method: Method, uri: &str) -> Response<Body> {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    router(demo(), SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap()
}

#[cfg(test)]
async fn get(uri: &str) -> Response<Body> {
    send(Method::GET, uri).await
}

#[cfg(test)]
fn header<'r>(response: &'r Response<Body>, name: &header::HeaderName) -> &'r str {
    response
        .headers()
        .get(name)
        .expect("the header is set")
        .to_str()
        .unwrap()
}

#[cfg(test)]
async fn text(response: Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn serves_a_page_at_its_clean_address() {
    let response = get("/blog/what-a-sitemap-does/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header(&response, &header::CONTENT_TYPE),
        "text/html; charset=utf-8"
    );
    assert_eq!(header(&response, &header::CONTENT_LENGTH), "28");
    assert_eq!(text(response).await, "<h1>What a sitemap does</h1>");
}

#[tokio::test]
async fn serves_each_file_with_its_media_type() {
    let robots = get("/robots.txt").await;
    assert_eq!(
        header(&robots, &header::CONTENT_TYPE),
        "text/plain; charset=utf-8"
    );
    let sitemap = get("/sitemap.xml").await;
    assert_eq!(
        header(&sitemap, &header::CONTENT_TYPE),
        "application/xml; charset=utf-8"
    );
}

#[tokio::test]
async fn redirects_an_address_missing_its_trailing_slash() {
    let response = get("/blog/what-a-sitemap-does?ref=newsletter").await;
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        header(&response, &header::LOCATION),
        "/blog/what-a-sitemap-does/?ref=newsletter"
    );
}

#[tokio::test]
async fn redirects_index_html_to_its_folder() {
    let response = get("/blog/index.html").await;
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(header(&response, &header::LOCATION), "/blog/");
}

#[tokio::test]
async fn serves_a_unicode_address_however_it_is_encoded() {
    let lower = get("/de/blog/%c3%bcber-seo/").await;
    assert_eq!(lower.status(), StatusCode::OK);
    assert_eq!(text(lower).await, "<h1>Über SEO</h1>");
    let redirect = get("/de/blog/%C3%BCber-seo").await;
    assert_eq!(
        header(&redirect, &header::LOCATION),
        "/de/blog/%C3%BCber-seo/"
    );
}

#[tokio::test]
async fn answers_an_unknown_address_with_the_not_found_page() {
    for uri in ["/blog/no-such-post/", "/blog/no-such-post", "/%ff/"] {
        let response = get(uri).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(
            header(&response, &header::CONTENT_TYPE),
            "text/html; charset=utf-8"
        );
        assert_eq!(text(response).await, "<h1>Page not found</h1>", "{uri}");
    }
}

#[tokio::test]
async fn never_redirects_to_another_site() {
    for uri in ["//evil.example", "/%2Fevil.example", "//blog"] {
        let response = get(uri).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert!(response.headers().get(header::LOCATION).is_none(), "{uri}");
    }
}

#[tokio::test]
async fn an_old_address_moves_permanently_to_its_new_one() {
    let path = |text: &str| PagePath::try_new(text).unwrap();
    let pages = demo().with_moves(vec![
        Moved::new(
            path("/blog/sitemaps-explained/"),
            path("/blog/what-a-sitemap-does/"),
        ),
        Moved::new(path("/blog/"), path("/blog/what-a-sitemap-does/")),
    ]);
    let request = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();
    let app = router(pages, SecurityHeaders::default());
    let moved = app
        .clone()
        .oneshot(request("/blog/sitemaps-explained/?ref=feed"))
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        header(&moved, &header::LOCATION),
        "/blog/what-a-sitemap-does/?ref=feed"
    );
    let page = app.oneshot(request("/blog/")).await.unwrap();
    assert_eq!(page.status(), StatusCode::OK, "a page wins over a move");
}

#[tokio::test]
async fn head_sends_the_headers_without_the_body() {
    let response = send(Method::HEAD, "/blog/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(header(&response, &header::CONTENT_LENGTH), "17");
    assert_eq!(text(response).await, "");
}

#[tokio::test]
async fn refuses_methods_that_change_things() {
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let response = send(method.clone(), "/blog/").await;
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method}"
        );
        assert_eq!(header(&response, &header::ALLOW), "GET, HEAD");
    }
}
