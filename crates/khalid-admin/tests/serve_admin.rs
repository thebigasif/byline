//! `/admin/` answers the built admin app, and says how to build it when
//! the binary holds none.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use khalid_admin::{Asset, router};
use tower::ServiceExt;

static BUILT: &[Asset] = &[
    Asset {
        path: "index.html",
        body: b"<!doctype html><title>Khalid admin</title>",
    },
    Asset {
        path: "assets/index-4f2a.js",
        body: b"console.log(1)",
    },
    Asset {
        path: "assets/index-4f2a.css",
        body: b"html{}",
    },
];

#[cfg(test)]
async fn get(files: &'static [Asset], path: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    let request = Request::get(path).body(Body::empty()).unwrap();
    let response = router(files).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn the_app_page_answers_at_admin_and_at_every_screen_address() {
    for path in ["/admin/", "/admin/sign-in", "/admin/posts/01JB"] {
        let (status, headers, body) = get(BUILT, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
        assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
        assert!(body.contains("Khalid admin"), "{path}");
    }
}

#[tokio::test]
async fn admin_without_a_slash_moves_to_admin_with_one() {
    let (status, headers, _) = get(BUILT, "/admin").await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(headers[header::LOCATION], "/admin/");
}

#[tokio::test]
async fn built_files_answer_with_their_type_and_are_cached_for_a_year() {
    let (status, headers, body) = get(BUILT, "/admin/assets/index-4f2a.js").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(
        headers[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(body, "console.log(1)");
    let (_, headers, _) = get(BUILT, "/admin/assets/index-4f2a.css").await;
    assert_eq!(headers[header::CONTENT_TYPE], "text/css; charset=utf-8");
}

#[tokio::test]
async fn a_missing_built_file_is_not_found_rather_than_the_app_page() {
    let (status, _, _) = get(BUILT, "/admin/assets/gone-0000.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn every_answer_limits_scripts_to_the_blog_itself() {
    for path in ["/admin/", "/admin/assets/index-4f2a.js"] {
        let (_, headers, _) = get(BUILT, path).await;
        let policy = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
        assert!(policy.contains("script-src 'self'"), "{policy}");
        assert!(policy.contains("frame-ancestors 'none'"), "{policy}");
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    }
}

#[tokio::test]
async fn pictures_may_come_from_unsplash_and_giphy_and_nowhere_else() {
    let (_, headers, _) = get(BUILT, "/admin/").await;
    let policy = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
    assert!(
        policy.contains("img-src 'self' data: https://images.unsplash.com https://*.giphy.com;"),
        "{policy}"
    );
    assert!(policy.contains("connect-src 'self';"), "{policy}");
}

#[tokio::test]
async fn a_binary_built_without_the_admin_says_how_to_build_it() {
    let (status, headers, body) = get(&[], "/admin/").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(headers[header::CONTENT_TYPE], "text/html; charset=utf-8");
    assert!(body.contains("npm run build"), "{body}");
    assert!(body.contains("<h1>"), "{body}");
}
