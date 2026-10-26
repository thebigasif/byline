//! The Content API under `/api/content/v1/`, and the `OpenAPI` document.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::openapi::OpenApi as Document;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::content_store::{ContentStore, ContentStoreError};
use crate::pagination::Meta;
use crate::problem::{Problem, ProblemType};
use crate::query::{ListQuery, RawListQuery};
use crate::resources::Post;

/// Where the Content API lives.
const BASE: &str = "/api/content/v1";

/// The store every route reads from.
type Store = Arc<dyn ContentStore>;

/// The document's title and the schemas every route shares.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Khalid API",
        description = "The Content API and the Admin API of a Khalid blog."
    ),
    components(schemas(Problem))
)]
struct ApiDoc;

/// The Content API routes and `/api/openapi.json`, with the `OpenAPI`
/// document the routes declare (RULES.md BL-API-4). Every route is
/// declared with `utoipa::path` and registered through `routes!`, so a
/// route cannot exist without its entry.
pub fn router(store: Store) -> (Router, Document) {
    let content = OpenApiRouter::new()
        .routes(routes!(browse_posts))
        .with_state(store);
    let (router, document) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest(BASE, content)
        .split_for_parts();
    let served = Json(document.clone());
    let router = router.route("/api/openapi.json", get(move || async move { served }));
    (router, document)
}

/// Posts, as every post response wraps them (`docs/standards/api.md`).
#[derive(Debug, Serialize, ToSchema)]
pub struct Posts {
    /// The posts on this page.
    posts: Vec<Post>,
    /// Pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
}

/// Lists published posts.
#[utoipa::path(
    get, path = "/posts/", tag = "posts", params(RawListQuery),
    responses(
        (status = 200, description = "One page of published posts", body = Posts),
        (status = 400, description = "A list parameter could not be read", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn browse_posts(
    State(store): State<Store>,
    Query(raw): Query<RawListQuery>,
) -> Result<Json<Posts>, Problem> {
    let query = ListQuery::try_from(&raw)?;
    check_fields(&query)?;
    let listed = store
        .posts(&query)
        .await
        .map_err(|error| unreadable(&error))?;
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    let formats = query.formats();
    let posts = listed
        .into_items()
        .into_iter()
        .map(|post| post.with_formats(formats));
    Ok(Json(Posts {
        posts: posts.collect(),
        meta: Some(meta),
    }))
}

/// Refuses fields a post does not have in `filter` and `order`.
fn check_fields(query: &ListQuery) -> Result<(), Problem> {
    let unknown = |parameter: &str, field: &str| {
        let message = UserMessage::new(
            format!("The {parameter} uses {field}, which is not a field here."),
            "The list cannot be sent until it is fixed.",
            format!(
                "Use one of these fields: {}.",
                Post::FILTER_FIELDS.join(", ")
            ),
        );
        Problem::new(ProblemType::InvalidQuery, &message).with_rule(parameter)
    };
    if let Some(filter) = query.filter() {
        filter
            .check_fields(&Post::FILTER_FIELDS)
            .map_err(|error| unknown("filter", error.field()))?;
    }
    match query
        .order()
        .iter()
        .find(|key| !Post::FILTER_FIELDS.contains(&key.field()))
    {
        Some(key) => Err(unknown("order", key.field())),
        None => Ok(()),
    }
}

/// Logs the store's failure with its causes and answers with a 500.
fn unreadable(error: &ContentStoreError) -> Problem {
    let error: &dyn std::error::Error = error;
    tracing::error!(error, "content store failed");
    let message = UserMessage::new(
        "The content could not be read.",
        "The server could not reach where its posts are kept.",
        "Try again in a minute, and check the server log if it keeps happening.",
    );
    Problem::new(ProblemType::Internal, &message)
}
