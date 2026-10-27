//! The site's media library in `khalid serve`: opened from `media/` with
//! its files on disk or in the bucket `[media.s3]` names, behind the Admin
//! API, and each local file served under the blog's address (ADR "the media
//! library keeps each file with its details").

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path as UrlPath, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, LOCATION, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use khalid_api::media_api::MediaLibrary;
use khalid_core::{Config, S3Keys, SystemClock, UserMessage};
use khalid_media::{ContentKey, FileAddress, Library, Storage};
use khalid_outbound::Outbound;

use crate::exit::Exit;

/// A file never changes at its address, since the key in it is a hash of
/// its bytes, so a browser may keep it for a year without asking again.
const CACHED: &str = "public, max-age=31536000, immutable";

/// No file from the library may run anything, even when opened directly.
const LOCKED: &str = "default-src 'none'; sandbox";

/// Opens the library of the `site` folder, in the bucket `config` names,
/// signed with `keys` and reached through `outbound`, or on disk. Returns
/// it for the Admin API, and the router that serves its files.
///
/// # Errors
///
/// Returns a message naming the file when an `item.toml` cannot be read or
/// is not valid, and the failure exit, so the server does not start.
pub(crate) async fn open(
    site: &Path,
    config: &Config,
    keys: Option<S3Keys>,
    outbound: &Outbound,
) -> Result<(MediaLibrary, Router), (UserMessage, Exit)> {
    let (library, api) = opened(site, config, keys, outbound)
        .await
        .map_err(|message| (message, Exit::Failure))?;
    Ok((api, router(library, config)))
}

async fn opened(
    site: &Path,
    config: &Config,
    keys: Option<S3Keys>,
    outbound: &Outbound,
) -> Result<(Arc<Library>, MediaLibrary), UserMessage> {
    let storage = match (&config.media().s3, keys) {
        (Some(bucket), Some(keys)) => {
            Storage::s3(bucket, keys, outbound.clone(), Arc::new(SystemClock))
        }
        _ => Storage::local(site),
    };
    let language = config.default_language();
    let opened = Library::open(site, storage, language, Arc::new(SystemClock)).await;
    let library = Arc::new(opened.map_err(|error| {
        UserMessage::new(
            format!("The media library could not be opened: {error}."),
            "Khalid keeps each file's details in media/<key>/item.toml and reads them all at start.",
            "Fix or remove the file the message names, then start khalid serve again.",
        )
    })?);
    let address = config.address().as_str().trim_end_matches('/');
    let api = MediaLibrary::new(
        Arc::clone(&library),
        &format!("{address}{}", config.mount()),
    );
    Ok((library, api))
}

/// Serves `library`'s files under `<mount>media/<key>/<name>`.
fn router(library: Arc<Library>, config: &Config) -> Router {
    let route = format!("{}media/{{key}}/{{name}}", config.mount());
    Router::new().route(&route, get(file)).with_state(library)
}

/// The file, or where it lives when it is in a bucket, or a plain 404.
async fn file(
    State(library): State<Arc<Library>>,
    UrlPath((key, name)): UrlPath<(String, String)>,
) -> Response {
    let Some(key) = ContentKey::parse(&key) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Some(item) = library.get(&key).filter(|item| item.name.as_str() == name)
        && let FileAddress::Elsewhere(url) = library.address(&item)
    {
        return match HeaderValue::from_str(&url) {
            Ok(url) => (StatusCode::PERMANENT_REDIRECT, [(LOCATION, url)]).into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        };
    }
    match library.read(&key, &name).await {
        Ok(Some((item, bytes))) => {
            let headers = [
                (
                    CONTENT_TYPE,
                    HeaderValue::from_static(item.media_type.mime()),
                ),
                (CACHE_CONTROL, HeaderValue::from_static(CACHED)),
                (X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
                (CONTENT_SECURITY_POLICY, HeaderValue::from_static(LOCKED)),
            ];
            (headers, bytes).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(%error, "a media file could not be read");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
