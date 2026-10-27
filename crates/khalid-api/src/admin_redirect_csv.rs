//! Redirect lists in and out of the Admin API as CSV, with the columns
//! `from,to,status,match` every importer writes (KH-SEO-8). Both routes
//! need `settings:write`.


use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::{Extension, Json, middleware};
use khalid_core::UserMessage;
use khalid_seo::{read_redirects_csv, write_redirects_csv};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::admin_redirects::{AdminRedirects, store_checked};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::redirect_desk::RedirectDesk;
use crate::redirect_store::Origin;

/// The CSV routes, behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(import_redirects))
        .routes(routes!(export_redirects))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// Stores every redirect in a CSV list, or none. The list is checked row
/// by row, then together with every stored redirect, so it can neither
/// take a used address nor close a loop. Needs `settings:write`.
#[utoipa::path(
    post, path = "/redirects/import/", tag = "admin redirects",
    security(("admin_key" = [])),
    request_body(
        content = String, content_type = "text/csv",
        description = "A header from,to,status,match (match may be left out), then one redirect a row",
        example = "from,to,status,match\n/blog/old-links/,/blog/why-internal-links-matter/,301,exact\n"
    ),
    responses(
        (status = 201, description = "Every redirect in the list, stored", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "An address in the list already has a redirect", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "A row cannot be read or followed, or the list would close a loop", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn import_redirects(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    body: String,
) -> Result<(StatusCode, Json<AdminRedirects>), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let entries = read_redirects_csv(&body).map_err(|error| {
        let message = UserMessage::new(
            format!("This list of redirects cannot be stored: {error}."),
            "Nothing from the list was stored, so no half list sends readers astray.",
            "Fix that row, keep the header from,to,status,match, and send the whole list again.",
        );
        Problem::new(ProblemType::InvalidBody, &message)
    })?;
    let added = store_checked(&desk, entries, Origin::Import).await?;
    Ok((StatusCode::CREATED, Json(added)))
}

/// Every stored redirect as a CSV list, oldest first, to
/// open in a spreadsheet or import elsewhere. Needs `settings:write`.
#[utoipa::path(
    get, path = "/redirects/export/", tag = "admin redirects",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The list, header first", body = String, content_type = "text/csv"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn export_redirects(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
) -> Result<([(header::HeaderName, &'static str); 2], String), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let stored = desk.store.list().await.map_err(|e| Problem::internal(&e))?;
    let entries: Vec<_> = stored.into_iter().map(|s| s.entry).collect();
    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"redirects.csv\"",
        ),
    ];
    Ok((headers, write_redirects_csv(&entries)))
}
