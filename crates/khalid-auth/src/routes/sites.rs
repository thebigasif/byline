//! The super-admin panel's routes: the sites this process serves, and
//! adding one by its domain (plan section 4.4, PRD KH-EDT-7).

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use khalid_api::problem::{Problem, ProblemType};
use khalid_api::session_check::StaffRoutes;
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::{cookie_token, problem, refused};
use crate::{Accounts, AddSiteError, CSRF_HEADER, Home, ListedSite, SiteDirectory, site_domain};

/// `staff` with the super-admin routes added to its signed-in routes: the
/// sites `directory` keeps, open to the super admins among `accounts`.
pub fn with_sites(
    staff: StaffRoutes,
    accounts: Accounts,
    directory: Arc<dyn SiteDirectory>,
) -> StaffRoutes {
    let sites = OpenApiRouter::new()
        .routes(routes!(list_sites, add_site))
        .with_state(Panel {
            accounts,
            directory,
        });
    StaffRoutes {
        open: staff.open,
        signed_in: staff.signed_in.merge(sites),
    }
}

/// What the routes need: who is signed in, and the sites.
#[derive(Clone)]
struct Panel {
    accounts: Accounts,
    directory: Arc<dyn SiteDirectory>,
}

/// One site as the Admin API shows it.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct SiteAnswer {
    /// The domain readers reach it on.
    domain: String,
    /// Whether it is the home site, where this panel lives.
    home: bool,
}

impl From<&ListedSite> for SiteAnswer {
    fn from(site: &ListedSite) -> Self {
        Self {
            domain: site.domain.as_str().to_owned(),
            home: site.home == Home::Yes,
        }
    }
}

/// Every site the process serves.
#[derive(Debug, Serialize, ToSchema)]
pub(super) struct SiteList {
    /// The sites, the home site first.
    sites: Vec<SiteAnswer>,
}

/// A site to add.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NewSite {
    /// The domain readers will reach it on, such as `news.example.com`.
    domain: String,
}

/// Lists every site this process serves. Only a super admin, signed in on
/// the home site, may see it; a key never may.
#[utoipa::path(
    get, path = "/sites/", tag = "sites",
    security(("staff_session" = [])),
    responses(
        (status = 200, description = "Every site, the home site first", body = SiteList),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "Not a super admin, or a key", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_sites(State(panel): State<Panel>, headers: HeaderMap) -> Response {
    if let Some(refusal) = refusal(&panel, &headers).await {
        return refusal;
    }
    match panel.directory.sites().await {
        Ok(listed) => {
            let sites = listed.iter().map(SiteAnswer::from).collect();
            Json(SiteList { sites }).into_response()
        }
        Err(error) => Problem::internal(&error).into_response(),
    }
}

/// Adds a site on a new domain and starts serving it, with its own
/// folder, database and staff. Only a super admin may.
#[utoipa::path(
    post, path = "/sites/", tag = "sites",
    security(("staff_session" = [])),
    request_body = NewSite,
    responses(
        (status = 201, description = "The site was made and is being served", body = SiteAnswer),
        (status = 401, description = "No live session", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "Not a super admin, or a key", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "A site already answers on this domain", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The body or the domain could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn add_site(
    State(panel): State<Panel>,
    headers: HeaderMap,
    body: Result<Json<NewSite>, JsonRejection>,
) -> Response {
    if let Some(refusal) = refusal(&panel, &headers).await {
        return refusal;
    }
    let domain = body
        .ok()
        .and_then(|Json(body)| site_domain(&body.domain).ok());
    let Some(domain) = domain else {
        return not_a_domain().into_response();
    };
    match panel.directory.add(&domain).await {
        Ok(site) => (StatusCode::CREATED, Json(SiteAnswer::from(&site))).into_response(),
        Err(AddSiteError::Taken) => taken().into_response(),
        Err(error) => Problem::internal(&error).into_response(),
    }
}

/// Why the request may not use the panel, or `None` when it comes from a
/// super admin: a live session with its CSRF token, for an account the
/// directory lists. A request with a key alone has no session, so it is
/// refused too.
async fn refusal(panel: &Panel, headers: &HeaderMap) -> Option<Response> {
    let Some(token) = cookie_token(headers) else {
        return Some(not_super_admin().into_response());
    };
    let signed_in = match panel.accounts.session(&token).await {
        Ok(signed_in) => signed_in,
        Err(error) => return Some(problem(&error)),
    };
    let csrf = headers
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok());
    if !csrf.is_some_and(|csrf| signed_in.csrf.matches(csrf)) {
        return Some(refused());
    }
    match panel
        .directory
        .is_super_admin(signed_in.account.email())
        .await
    {
        Ok(true) => None,
        Ok(false) => Some(not_super_admin().into_response()),
        Err(error) => Some(Problem::internal(&error).into_response()),
    }
}

fn not_super_admin() -> Problem {
    let message = UserMessage::new(
        "Only a super admin may see and add the sites on this server.",
        "Adding a site gives it a domain on this server, so only the people the server lists may.",
        "Sign in on the home site with a super-admin account, or ask the server's owner to add you.",
    );
    Problem::new(ProblemType::Forbidden, &message)
}

fn not_a_domain() -> Problem {
    let message = UserMessage::new(
        "The domain could not be read.",
        "Each site answers on its own domain, such as news.example.com.",
        "Send JSON with domain, written with letters, digits, dots and dashes only.",
    );
    Problem::new(ProblemType::InvalidBody, &message)
}

fn taken() -> Problem {
    let message = UserMessage::new(
        "A site on this server already answers on this domain.",
        "Each domain leads to one site, so readers always reach the same one.",
        "Choose another domain, or open the site that has it.",
    );
    Problem::new(ProblemType::Conflict, &message)
}
