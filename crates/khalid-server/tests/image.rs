//! An image or other file beside a post is sent with the media type its
//! name gives, and is never compressed again, because images already are.

use axum::body::Body;
use axum::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, ETAG};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, router};
use tower::ServiceExt;

/// A PNG's eight signature bytes, then a run that Brotli would shrink.
#[cfg(test)]
fn png() -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend([0_u8; 4096]);
    bytes
}

#[tokio::test]
async fn an_image_is_sent_with_its_type_and_without_brotli() {
    let path = PagePath::try_new("/blog/fixing-slow-pages/page-weight.png").unwrap();
    let pages = Pages::new(vec![Page::new(path.clone(), MediaType::Png, png())], "").unwrap();
    let request = Request::get(path.as_str())
        .header(ACCEPT_ENCODING, "br")
        .body(Body::empty())
        .unwrap();
    let response = router(pages, SecurityHeaders::default())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
    assert!(response.headers().contains_key(ETAG));
    assert!(!response.headers().contains_key(CONTENT_ENCODING));
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body, png());
}

#[test]
fn a_file_name_gives_its_media_type() {
    for (name, kind) in [
        ("photo.jpg", MediaType::Jpeg),
        ("photo.JPEG", MediaType::Jpeg),
        ("chart.png", MediaType::Png),
        ("chart.webp", MediaType::Webp),
        ("chart.avif", MediaType::Avif),
        ("loop.gif", MediaType::Gif),
        ("logo.svg", MediaType::Svg),
        ("guide.pdf", MediaType::Pdf),
        ("notes.txt", MediaType::PlainText),
        ("data.json", MediaType::Json),
        ("archive.zip", MediaType::Binary),
        ("no-extension", MediaType::Binary),
    ] {
        assert_eq!(MediaType::for_file(name), kind, "{name}");
    }
}

#[test]
fn each_file_kind_names_its_type() {
    assert_eq!(MediaType::Png.as_header(), "image/png");
    assert_eq!(MediaType::Jpeg.as_header(), "image/jpeg");
    assert_eq!(MediaType::Webp.as_header(), "image/webp");
    assert_eq!(MediaType::Avif.as_header(), "image/avif");
    assert_eq!(MediaType::Gif.as_header(), "image/gif");
    assert_eq!(MediaType::Svg.as_header(), "image/svg+xml");
    assert_eq!(MediaType::Pdf.as_header(), "application/pdf");
    assert_eq!(MediaType::Binary.as_header(), "application/octet-stream");
}
