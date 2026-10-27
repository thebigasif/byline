//! The redirect routes of the Admin API under `/api/admin/v1/redirects/`
//! (KH-SEO-8). Every route needs `settings:write`.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Utc};
use khalid_core::UserMessage;
use khalid_seo::{
    Matched, Matching, Redirect, RedirectColumns, RedirectEntry, RedirectTable, SeoError, UrlPath,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{KeyStore, Principal, Scope};
use crate::problem::{Problem, ProblemType};
use crate::redirect_desk::RedirectDesk;
use crate::redirect_store::{Origin, RedirectStoreError, StoredRedirect};

/// The redirect routes, each declared with `utoipa::path` (RULES.md
/// BL-API-4), behind an integration key or staff token.
pub(crate) fn routes(desk: RedirectDesk, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_redirects, create_redirect))
        .routes(routes!(remove_redirect))
        .routes(routes!(explain_redirect))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// A stored redirect as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminRedirect {
    /// The identity to remove it by.
    #[schema(example = 12)]
    id: i64,
    /// The path it starts from, or a regular expression over the path.
    #[schema(example = "/blog/old-links/")]
    from: String,
    /// Where a move leads: a path, a web address, or a pattern's template
    /// with `$1` for its first group. Empty for 410 and 451.
    #[schema(example = "/blog/why-internal-links-matter/")]
    to: Option<String>,
    /// 301, 302, 307, 410 or 451.
    #[schema(example = 301)]
    status: u16,
    /// `exact` or `regex`.
    #[serde(rename = "match")]
    #[schema(example = "exact")]
    matching: &'static str,
    /// `manual`, `import` or `slug_change`.
    #[schema(example = "manual")]
    origin: &'static str,
    /// When it was made.
    created_at: DateTime<Utc>,
}

impl From<&StoredRedirect> for AdminRedirect {
    fn from(stored: &StoredRedirect) -> Self {
        let columns = stored.entry.columns();
        Self {
            id: stored.id,
            from: columns.from,
            to: columns.to,
            status: columns.status,
            matching: columns.matching.as_str(),
            origin: stored.origin.as_str(),
            created_at: stored.created_at,
        }
    }
}

/// Redirects as the Admin API wraps them.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminRedirects {
    /// The redirects, oldest first.
    redirects: Vec<AdminRedirect>,
}

/// Lists every stored redirect, oldest first. Aliases in front matter are
/// not listed; they live in their posts. Needs `settings:write`.
#[utoipa::path(
    get, path = "/redirects/", tag = "admin redirects",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "Every stored redirect", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_redirects(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<AdminRedirects>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let stored = desk
        .store
        .list()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let redirects = stored.iter().map(AdminRedirect::from).collect();
    Ok(Json(AdminRedirects { redirects }))
}

/// One new redirect, in the columns [`AdminRedirect`] shows.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewRedirect {
    /// The path it starts from, or a regular expression over the path.
    #[schema(example = "/blog/old-links/")]
    from: String,
    /// Where a move leads. Leave it out for 410 and 451.
    #[serde(default)]
    #[schema(example = "/blog/why-internal-links-matter/")]
    to: Option<String>,
    /// 301, 302, 307, 410 or 451.
    #[schema(example = 301)]
    status: u16,
    /// `exact`, the default, or `regex`.
    #[serde(rename = "match", default)]
    #[schema(example = "exact")]
    matching: Option<String>,
}

/// The body of a new redirect: exactly one.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct NewRedirects {
    /// The one redirect to store.
    redirects: Vec<NewRedirect>,
}

/// Stores one redirect made by hand. It is checked against every stored
/// redirect, so it can neither take an address that has one nor close a
/// loop. Needs `settings:write`.
#[utoipa::path(
    post, path = "/redirects/", tag = "admin redirects",
    security(("admin_key" = [])),
    request_body = NewRedirects,
    responses(
        (status = 201, description = "The stored redirect", body = AdminRedirects),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The address already has a redirect", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The redirect cannot be followed or would close a loop", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_redirect(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    body: Result<Json<NewRedirects>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminRedirects>), Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let Json(body) = body.map_err(|rejection| unstorable(&rejection.body_text()))?;
    let [new] = <[NewRedirect; 1]>::try_from(body.redirects)
        .map_err(|_| unstorable("redirects must hold exactly one redirect"))?;
    let matching = new.matching.as_deref().unwrap_or("exact");
    let matching =
        Matching::from_name(matching).ok_or_else(|| unstorable("match must be exact or regex"))?;
    let columns = RedirectColumns::new(&new.from, new.to.as_deref(), new.status, matching);
    let entry = RedirectEntry::from_columns(&columns).map_err(|e| unstorable(&e.to_string()))?;
    let stored = desk.store.list().await.map_err(|e| Problem::internal(&e))?;
    let all = stored.into_iter().map(|s| s.entry).chain([entry.clone()]);
    RedirectTable::from_entries(all.collect()).map_err(|error| refused(&error))?;
    let added = desk
        .store
        .add(vec![entry], Origin::Manual, desk.clock.now())
        .await
        .map_err(|error| match error {
            RedirectStoreError::Taken { from } => taken(&from),
            error => Problem::internal(&error),
        })?;
    let redirects = added.iter().map(AdminRedirect::from).collect();
    Ok((StatusCode::CREATED, Json(AdminRedirects { redirects })))
}

/// Removes one stored redirect. Needs `settings:write`.
#[utoipa::path(
    delete, path = "/redirects/{id}/", tag = "admin redirects",
    security(("admin_key" = [])),
    params(("id" = i64, Path, description = "The redirect's identity")),
    responses(
        (status = 204, description = "The redirect is gone"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No redirect has this identity", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn remove_redirect(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    // An identity that is not a number names no redirect, so it gets the
    // same 404 as one already removed.
    let removed = match id.parse::<i64>() {
        Ok(id) => desk
            .store
            .remove(id)
            .await
            .map_err(|e| Problem::internal(&e))?,
        Err(_) => false,
    };
    if !removed {
        let message = UserMessage::new(
            "No redirect has this identity.",
            "It may have been removed already.",
            "List the redirects to find the one you want.",
        );
        return Err(Problem::new(ProblemType::NotFound, &message));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The address to explain.
#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct ExplainQuery {
    /// The path, from the site root, such as `/blog/old-links/`.
    path: String,
}

/// One hop of a chain in the debug view.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ExplainedHop {
    /// The address this hop starts from.
    from: String,
    /// The identity of the stored redirect that answered it.
    redirect: Option<i64>,
    /// What that redirect answers on its own.
    status: u16,
    /// Where it leads, for a move.
    to: Option<String>,
}

/// Why an address answers what it answers: the debug view.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RedirectExplanation {
    /// The address asked about.
    path: String,
    /// The identity of the stored redirect it matched, if any.
    matched: Option<i64>,
    /// Each hop from the address to the end of its chain.
    hops: Vec<ExplainedHop>,
    /// The one status a visitor gets, if a redirect matched.
    status: Option<u16>,
    /// Where a visitor is sent in one step, for a move.
    to: Option<String>,
    /// The same, in plain words.
    text: String,
}

/// Explains which stored redirect an address matches, every hop of its
/// chain, and the one step a visitor gets. Aliases in front matter and
/// pages answer before stored redirects, so they are not shown here.
/// Needs `settings:write`.
#[utoipa::path(
    get, path = "/redirects/explain/", tag = "admin redirects",
    security(("admin_key" = [])),
    params(ExplainQuery),
    responses(
        (status = 200, description = "Why the address answers what it does", body = RedirectExplanation),
        (status = 400, description = "The path is not a clean path from the site root", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn explain_redirect(
    State(desk): State<RedirectDesk>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<ExplainQuery>, QueryRejection>,
) -> Result<Json<RedirectExplanation>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let path = query
        .ok()
        .and_then(|Query(query)| UrlPath::try_new(&query.path).ok())
        .ok_or_else(bad_path)?;
    let stored = desk.store.list().await.map_err(|e| Problem::internal(&e))?;
    // The table tries exact rules first, then patterns, and numbers them
    // in that order.
    let (exact, patterns): (Vec<_>, Vec<_>) = stored
        .iter()
        .partition(|s| matches!(s.entry, RedirectEntry::Exact(_)));
    let ids: Vec<i64> = exact.iter().chain(&patterns).map(|s| s.id).collect();
    let entries = stored.iter().map(|s| s.entry.clone()).collect();
    let table = RedirectTable::from_entries(entries).map_err(|e| Problem::internal(&e))?;
    let explained = table.explain(&path);
    let id = |rule: usize| ids.get(rule).copied();
    let matched = explained
        .matched
        .as_ref()
        .and_then(|matched| match matched {
            Matched::Exact { rule } | Matched::Pattern { rule, .. } => id(*rule),
            _ => None,
        });
    let hops = explained.hops.iter().map(|hop| ExplainedHop {
        from: hop.from.to_string(),
        redirect: id(hop.rule),
        status: hop.answer.status(),
        to: hop.answer.location().map(ToString::to_string),
    });
    let answer = explained.answer.as_ref();
    Ok(Json(RedirectExplanation {
        path: path.to_string(),
        matched,
        hops: hops.collect(),
        status: answer.map(Redirect::status),
        to: answer.and_then(Redirect::location).map(ToString::to_string),
        text: explained.to_string(),
    }))
}

/// The 400 for an address that is not a clean path.
fn bad_path() -> Problem {
    let message = UserMessage::new(
        "The path to explain is not a clean path from the site root.",
        "Only such a path can match a redirect.",
        "Send path as it appears after the domain, starting with /, such as /blog/old-links/.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("path")
}

/// The 422 for a redirect that cannot be followed.
fn unstorable(reason: &str) -> Problem {
    let message = UserMessage::new(
        format!("This redirect cannot be stored: {reason}."),
        "A redirect that cannot be followed would send readers nowhere.",
        "Give a from path starting with /, a status of 301, 302, 307, 410 or 451, \
         a to address for a move and none for 410 or 451, and match exact or regex.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

/// The answer when the whole table refuses the new redirect.
fn refused(error: &SeoError) -> Problem {
    match error {
        SeoError::RedirectDuplicate { from } => taken(from),
        error => {
            let message = UserMessage::new(
                format!("This redirect cannot be stored: {error}."),
                "Readers and search engines would go round in a loop and never reach a page.",
                "Point it at the final address, or remove the redirect that leads back.",
            );
            Problem::new(ProblemType::InvalidBody, &message)
        }
    }
}

/// The 409 for an address that already has a redirect.
fn taken(from: &str) -> Problem {
    let message = UserMessage::new(
        format!("The address {from} already has a redirect."),
        "One address can lead to only one place.",
        "Remove the redirect it has first, then add the new one.",
    );
    Problem::new(ProblemType::Conflict, &message)
}
