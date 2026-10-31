//! Search Console follow-ups through the Admin API (GEO M3): the long
//! searches that showed each post and the heading that answers each. The
//! route needs `settings:write`, because the numbers come from the owner's
//! Google account.

use std::sync::Arc;

use axum::extract::State;
use axum::{Extension, Json, middleware};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::follow_ups::{FollowUpError, FollowUpFinder, FollowUps};
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};

/// The finder the route calls.
type Finder = Arc<dyn FollowUpFinder>;

/// The follow-up route, behind an integration key or staff token.
pub(crate) fn routes(finder: Finder, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(follow_ups))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(finder)
}

/// Lists the long searches, of 8 words or more, that showed each post in
/// the last 28 days whose numbers are final, with the heading of the post
/// that answers each. Google is asked on each request, so the answer can
/// take some seconds. Needs `settings:write`.
#[utoipa::path(
    get, path = "/search-console/follow-ups/", tag = "admin search console",
    security(("admin_key" = [])),
    responses(
        (status = 200, description = "The long searches per post", body = FollowUps),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The site is not connected to Search Console", body = Problem, content_type = "application/problem+json"),
        (status = 502, description = "Google refused or could not be reached", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn follow_ups(
    State(finder): State<Finder>,
    Extension(principal): Extension<Principal>,
) -> Result<Json<FollowUps>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    match finder.follow_ups().await {
        Ok(found) => Ok(Json(found)),
        Err(FollowUpError::NotConnected(message)) => {
            Err(Problem::new(ProblemType::NotSetUp, &message).with_rule("search_console"))
        }
        Err(FollowUpError::Google(message)) => Err(Problem::new(ProblemType::Upstream, &message)),
        Err(error) => Err(Problem::internal(&error)),
    }
}
