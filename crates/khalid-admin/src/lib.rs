//! The admin app, built from `admin/` by Vite and embedded in the binary,
//! answered under `/admin/` (plan section 4.6). It talks only to the Admin
//! and Content APIs, which `khalid serve` mounts beside it.

use axum::Router;
use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;

/// One file of the built admin, by its path under `admin/dist/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asset {
    /// Where it sits under `admin/dist/`, such as `assets/index-4f2a.js`.
    pub path: &'static str,
    /// The file's bytes.
    pub body: &'static [u8],
}

mod built {
    include!(concat!(env!("OUT_DIR"), "/admin_files.rs"));
}

/// The admin's own policy: its scripts, styles and API calls come from the
/// blog itself, and no other site may frame it.
const POLICY: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
    img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; \
    form-action 'self'";

/// Vite names every file under `assets/` by a hash of its content, so a
/// browser may keep it; the page itself is checked on every visit.
const FOR_A_YEAR: &str = "public, max-age=31536000, immutable";
const CHECK_EACH_TIME: &str = "no-cache";

const NOT_BUILT: &str = "<!doctype html>\n<html lang=\"en\">\n<meta charset=\"utf-8\">\n\
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
    <title>The admin is not in this copy of Khalid</title>\n\
    <h1>The admin is not in this copy of Khalid</h1>\n\
    <p>This program was built without the admin app, so there is nothing to show here. \
    The blog and its APIs work as usual.</p>\n\
    <p>To add it, run <code>npm ci</code> and <code>npm run build</code> in the \
    <code>admin</code> folder, then build Khalid again.</p>\n";

/// The files `npm run build` left in `admin/dist/` when this binary was
/// built; none when it had not run.
pub fn built() -> &'static [Asset] {
    built::FILES
}

/// The routes that answer the admin: `/admin` moves to `/admin/`, a built
/// file answers as itself, and any other address under `/admin/` gets the
/// app's page, whose router shows the screen for it.
pub fn router(files: &'static [Asset]) -> Router {
    Router::new()
        .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
        .route("/admin/", get(move || async move { page(files) }))
        .route(
            "/admin/{*path}",
            get(move |Path(path): Path<String>| async move { file(files, &path) }),
        )
}

fn file(files: &[Asset], path: &str) -> Response {
    match files.iter().find(|asset| asset.path == path) {
        Some(asset) if path.starts_with("assets/") => answer(asset, FOR_A_YEAR),
        Some(asset) => answer(asset, CHECK_EACH_TIME),
        None if path.starts_with("assets/") => StatusCode::NOT_FOUND.into_response(),
        None => page(files),
    }
}

fn page(files: &[Asset]) -> Response {
    if let Some(asset) = files.iter().find(|asset| asset.path == "index.html") {
        return answer(asset, CHECK_EACH_TIME);
    }
    let headers = headers("text/html; charset=utf-8", CHECK_EACH_TIME);
    (StatusCode::SERVICE_UNAVAILABLE, headers, NOT_BUILT).into_response()
}

fn answer(asset: &Asset, cache: &'static str) -> Response {
    (headers(content_type(asset.path), cache), asset.body).into_response()
}

fn headers(
    content_type: &'static str,
    cache: &'static str,
) -> [(header::HeaderName, &'static str); 5] {
    [
        (header::CONTENT_TYPE, content_type),
        (header::CACHE_CONTROL, cache),
        (header::CONTENT_SECURITY_POLICY, POLICY),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
    ]
}

/// The type of a built file, from its extension.
fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
