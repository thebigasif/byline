//! The Content API under `/api/content/v1/`.

use std::sync::Arc;

use axum::extract::{OriginalUri, Path, Query, State};
use axum::middleware;
use axum::{Json, Router};
use khalid_core::{Slug, UserMessage};
use serde::Serialize;
use utoipa::openapi::OpenApi as Document;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::{ApiDoc, finish};
use crate::api_version::negotiate_version;
use crate::content_key::require_content_key;
use crate::content_store::{ContentStore, PostKey, SiteResources, select};
use crate::etag::tag_and_revalidate;
use crate::keys::KeyStore;
use crate::pagination::Meta;
use crate::problem::{Problem, ProblemType};
use crate::query::{ListQuery, RawListQuery};
use crate::resources::{
    Author, Collection, ContentCollection, Listable, Menu, Post, Settings, Term,
};

/// Where the Content API lives.
pub(crate) const BASE: &str = "/api/content/v1";

/// The store every route reads from.
type Store = Arc<dyn ContentStore>;

/// The Content API alone and `/api/openapi.json`. [`api::router`] serves
/// every API behind one document.
///
/// [`api::router`]: crate::api::router
pub fn router(store: Store, keys: Arc<dyn KeyStore>) -> (Router, Document) {
    finish(OpenApiRouter::with_openapi(ApiDoc::openapi()).nest(BASE, routes(store, keys)))
}

/// The Content API routes, each declared with `utoipa::path` and
/// registered through `routes!`, so a route cannot exist without its
/// `OpenAPI` entry (RULES.md BL-API-4). Every route needs a content key.
pub(crate) fn routes(store: Store, keys: Arc<dyn KeyStore>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(browse_posts))
        .routes(routes!(read_post))
        .routes(routes!(read_post_by_slug))
        .routes(routes!(browse_pages))
        .routes(routes!(read_page))
        .routes(routes!(read_page_by_slug))
        .routes(routes!(browse_glossary))
        .routes(routes!(read_glossary_term))
        .routes(routes!(read_glossary_term_by_slug))
        .routes(routes!(browse_case_studies))
        .routes(routes!(read_case_study))
        .routes(routes!(read_case_study_by_slug))
        .routes(routes!(browse_changelog))
        .routes(routes!(read_changelog_entry))
        .routes(routes!(read_changelog_entry_by_slug))
        .routes(routes!(browse_authors))
        .routes(routes!(read_author))
        .routes(routes!(read_author_by_slug))
        .routes(routes!(browse_tags))
        .routes(routes!(read_tag))
        .routes(routes!(read_tag_by_slug))
        .routes(routes!(browse_categories))
        .routes(routes!(read_category))
        .routes(routes!(read_category_by_slug))
        .routes(routes!(browse_collections))
        .routes(routes!(read_collection))
        .routes(routes!(read_collection_by_slug))
        .routes(routes!(browse_navigation))
        .routes(routes!(read_menu))
        .routes(routes!(read_menu_by_slug))
        .routes(routes!(read_settings))
        .route_layer(middleware::from_fn_with_state(keys, require_content_key))
        .route_layer(middleware::from_fn(tag_and_revalidate))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(store)
}

/// Declares one resource's envelope and its three routes: the list, one
/// entry by identity and one by slug. Each route keeps its own `OpenAPI`
/// entry and operation id; the work is in `$list` and `$one`, which read
/// `$source`: [`browse`] and [`one`] for a collection, [`browse_site`] and
/// [`one_in_site`] for a list in [`SiteResources`].
macro_rules! collection_routes {
    (
        $Item:ty, $source:expr, $list:ident, $one:ident, $Envelope:ident, $key:ident, $tag:tt,
        $browse:ident = $list_path:tt,
        $read:ident = $id_path:tt,
        $read_by_slug:ident = $slug_path:tt $(,)?
    ) => {
        #[doc = concat!("Entries of `", $tag, "`, as every response wraps them.")]
        #[derive(Debug, Serialize, ToSchema)]
        pub struct $Envelope {
            /// The entries on this page, or the one entry asked for.
            $key: Vec<$Item>,
            /// Pagination, on lists only.
            #[serde(skip_serializing_if = "Option::is_none")]
            meta: Option<Meta>,
        }

        #[doc = concat!("Lists the published entries of `", $tag, "`.")]
        #[utoipa::path(
            get, path = $list_path, tag = $tag, params(RawListQuery),
            security(("content_key" = []), ("content_key_query" = [])),
            responses(
                (status = 200, description = "One page of published entries", body = $Envelope),
                (status = 400, description = "A list parameter could not be read", body = Problem, content_type = "application/problem+json"),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $browse(
            State(store): State<Store>,
            Query(raw): Query<RawListQuery>,
        ) -> Result<Json<$Envelope>, Problem> {
            let (entries, meta) = $list(&store, $source, &raw).await?;
            Ok(Json($Envelope { $key: entries, meta: Some(meta) }))
        }

        #[doc = concat!("Reads one published entry of `", $tag, "` by its identity.")]
        #[utoipa::path(
            get, path = $id_path, tag = $tag,
            security(("content_key" = []), ("content_key_query" = [])),
            params(("id" = String, Path, description = "The entry's identity")),
            responses(
                (status = 200, description = "The entry", body = $Envelope),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
                (status = 404, description = "No published entry has this identity", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $read(
            State(store): State<Store>,
            Path(id): Path<String>,
            OriginalUri(uri): OriginalUri,
        ) -> Result<Json<$Envelope>, Problem> {
            let entry = $one(&store, $source, &PostKey::Id(id), uri.path()).await?;
            Ok(Json($Envelope { $key: vec![entry], meta: None }))
        }

        #[doc = concat!("Reads one published entry of `", $tag, "` by its slug.")]
        #[utoipa::path(
            get, path = $slug_path, tag = $tag,
            security(("content_key" = []), ("content_key_query" = [])),
            params(("slug" = String, Path, description = "The entry's slug")),
            responses(
                (status = 200, description = "The entry", body = $Envelope),
                (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
                (status = 404, description = "No published entry has this slug", body = Problem, content_type = "application/problem+json"),
            )
        )]
        async fn $read_by_slug(
            State(store): State<Store>,
            Path(slug): Path<String>,
            OriginalUri(uri): OriginalUri,
        ) -> Result<Json<$Envelope>, Problem> {
            let slug = Slug::try_new(&slug).map_err(|_| not_found(uri.path()))?;
            let entry = $one(&store, $source, &PostKey::Slug(slug), uri.path()).await?;
            Ok(Json($Envelope { $key: vec![entry], meta: None }))
        }
    };
}

collection_routes!(
    Post,
    Collection::Posts,
    browse,
    one,
    Posts,
    posts,
    "posts",
    browse_posts = "/posts/",
    read_post = "/posts/{id}/",
    read_post_by_slug = "/posts/slug/{slug}/",
);
collection_routes!(
    Post,
    Collection::Pages,
    browse,
    one,
    Pages,
    pages,
    "pages",
    browse_pages = "/pages/",
    read_page = "/pages/{id}/",
    read_page_by_slug = "/pages/slug/{slug}/",
);
collection_routes!(
    Post,
    Collection::Glossary,
    browse,
    one,
    GlossaryTerms,
    glossary,
    "glossary",
    browse_glossary = "/glossary/",
    read_glossary_term = "/glossary/{id}/",
    read_glossary_term_by_slug = "/glossary/slug/{slug}/",
);
collection_routes!(
    Post,
    Collection::CaseStudies,
    browse,
    one,
    CaseStudies,
    case_studies,
    "case-studies",
    browse_case_studies = "/case-studies/",
    read_case_study = "/case-studies/{id}/",
    read_case_study_by_slug = "/case-studies/slug/{slug}/",
);
collection_routes!(
    Post,
    Collection::Changelog,
    browse,
    one,
    ChangelogEntries,
    changelog,
    "changelog",
    browse_changelog = "/changelog/",
    read_changelog_entry = "/changelog/{id}/",
    read_changelog_entry_by_slug = "/changelog/slug/{slug}/",
);
collection_routes!(
    Author,
    SiteResources::authors,
    browse_site,
    one_in_site,
    Authors,
    authors,
    "authors",
    browse_authors = "/authors/",
    read_author = "/authors/{id}/",
    read_author_by_slug = "/authors/slug/{slug}/",
);
collection_routes!(
    Term,
    SiteResources::tags,
    browse_site,
    one_in_site,
    Tags,
    tags,
    "tags",
    browse_tags = "/tags/",
    read_tag = "/tags/{id}/",
    read_tag_by_slug = "/tags/slug/{slug}/",
);
collection_routes!(
    Term,
    SiteResources::categories,
    browse_site,
    one_in_site,
    Categories,
    categories,
    "categories",
    browse_categories = "/categories/",
    read_category = "/categories/{id}/",
    read_category_by_slug = "/categories/slug/{slug}/",
);
collection_routes!(
    ContentCollection,
    SiteResources::collections,
    browse_site,
    one_in_site,
    Collections,
    collections,
    "collections",
    browse_collections = "/collections/",
    read_collection = "/collections/{id}/",
    read_collection_by_slug = "/collections/slug/{slug}/",
);
collection_routes!(
    Menu,
    SiteResources::menus,
    browse_site,
    one_in_site,
    Navigation,
    navigation,
    "navigation",
    browse_navigation = "/navigation/",
    read_menu = "/navigation/{id}/",
    read_menu_by_slug = "/navigation/slug/{slug}/",
);

/// The site's settings, as every response wraps them: a list of one.
#[derive(Debug, Serialize, ToSchema)]
pub struct SiteSettings {
    /// The settings.
    settings: Vec<Settings>,
}

/// Reads the site's public settings: its name, address and languages.
#[utoipa::path(
    get, path = "/settings/", tag = "settings",
    security(("content_key" = []), ("content_key_query" = [])),
    responses(
        (status = 200, description = "The settings", body = SiteSettings),
        (status = 401, description = "No valid content key", body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "The site has no settings to show yet", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_settings(
    State(store): State<Store>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<SiteSettings>, Problem> {
    let site = store
        .site()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let settings = site.settings().ok_or_else(|| not_found(uri.path()))?;
    Ok(Json(SiteSettings {
        settings: vec![settings.clone()],
    }))
}

/// One page of `collection` for a list request, with its pagination.
async fn browse(
    store: &Store,
    collection: Collection,
    raw: &RawListQuery,
) -> Result<(Vec<Post>, Meta), Problem> {
    let query = ListQuery::try_from(raw)?;
    check_sparse(&query)?;
    check_fields(&query, Post::FIELDS)?;
    let listed = store
        .list(collection, &query)
        .await
        .map_err(|error| Problem::internal(&error))?;
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    let formats = query.formats();
    let entries = listed
        .into_items()
        .into_iter()
        .map(|entry| entry.with_formats(formats));
    Ok((entries.collect(), meta))
}

/// One entry of `collection`, or a 404 problem about `path`.
async fn one(
    store: &Store,
    collection: Collection,
    key: &PostKey,
    path: &str,
) -> Result<Post, Problem> {
    let entry = store
        .find(collection, key)
        .await
        .map_err(|error| Problem::internal(&error))?;
    entry.ok_or_else(|| not_found(path))
}

/// One page of a site list, `pick`ed from the store's [`SiteResources`].
async fn browse_site<T: Listable>(
    store: &Store,
    pick: fn(&SiteResources) -> &[T],
    raw: &RawListQuery,
) -> Result<(Vec<T>, Meta), Problem> {
    let query = ListQuery::try_from(raw)?;
    check_sparse(&query)?;
    check_fields(&query, T::FIELDS)?;
    let site = store
        .site()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let listed = select(pick(&site).iter(), &query);
    let meta = Meta::new(query.page(), query.limit(), listed.total());
    Ok((listed.into_items(), meta))
}

/// One entry of a site list, or a 404 problem about `path`.
async fn one_in_site<T: Listable>(
    store: &Store,
    pick: fn(&SiteResources) -> &[T],
    key: &PostKey,
    path: &str,
) -> Result<T, Problem> {
    let site = store
        .site()
        .await
        .map_err(|error| Problem::internal(&error))?;
    let found = pick(&site).iter().find(|entry| match key {
        PostKey::Id(id) => entry.id() == id,
        PostKey::Slug(slug) => entry.slug() == slug,
    });
    found.cloned().ok_or_else(|| not_found(path))
}

/// Refuses `include` and `fields`, which are accepted on the command line
/// but not answered yet (audit F6, G23). Saying so is kinder than sending
/// everything and letting a caller believe it asked for less.
pub(crate) fn check_sparse(query: &ListQuery) -> Result<(), Problem> {
    for (parameter, asked) in [("include", query.include()), ("fields", query.fields())] {
        if asked.is_empty() {
            continue;
        }
        let message = UserMessage::new(
            format!("The {parameter} parameter is not answered yet."),
            "It is accepted so future callers are not broken, but every field is always sent.",
            format!("Drop {parameter} for now; the whole entry is the only answer."),
        );
        return Err(Problem::new(ProblemType::InvalidQuery, &message).with_rule(parameter));
    }
    Ok(())
}

/// Refuses fields the resource does not have in `filter` and `order`.
pub(crate) fn check_fields(query: &ListQuery, fields: &[&str]) -> Result<(), Problem> {
    let unknown = |parameter: &str, field: &str| {
        let message = UserMessage::new(
            format!("The {parameter} uses {field}, which is not a field here."),
            "The list cannot be sent until it is fixed.",
            format!("Use one of these fields: {}.", fields.join(", ")),
        );
        Problem::new(ProblemType::InvalidQuery, &message).with_rule(parameter)
    };
    if let Some(filter) = query.filter() {
        filter
            .check_fields(fields)
            .map_err(|error| unknown("filter", error.field()))?;
    }
    match query
        .order()
        .iter()
        .find(|key| !fields.contains(&key.field()))
    {
        Some(key) => Err(unknown("order", key.field())),
        None => Ok(()),
    }
}

fn not_found(path: &str) -> Problem {
    let message = UserMessage::new(
        "Nothing published is at this address.",
        "It may still be a draft, or its slug may have changed.",
        "Check the address, or fetch the list it belongs to, such as /api/content/v1/posts/.",
    );
    Problem::new(ProblemType::NotFound, &message).with_instance(path)
}
