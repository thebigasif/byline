//! The calendar route of the Admin API under `/api/admin/v1/calendar/`:
//! drafts, scheduled posts and published posts by day (KH-EDT-4, plan
//! section 4.6). Moving a post to another day is an edit to the post, so
//! the calendar only reads.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::{Extension, Json, middleware};
use chrono::{DateTime, Days, NaiveDate, Utc};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::post_store::PostStoreError;
use crate::problem::{Problem, ProblemType};
use crate::publish_door::PublishDoor;
use crate::query::{ListQuery, RawListQuery};
use crate::resources::{AdminPost, PostStatus, ReviewState};

/// The most days one calendar request may span from its first to its last
/// day: about a quarter, so a request reads a bounded number of posts.
const MAX_CALENDAR_DAYS: i64 = 92;

/// How many rows one read of the posts list asks for: the most it sends.
const PAGE: u32 = 100;

/// The calendar route, behind an integration key or staff token.
pub(crate) fn routes(door: Arc<PublishDoor>, keys: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(read_calendar))
        .route_layer(middleware::from_fn_with_state(keys, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(door)
}

/// The days a calendar covers, both included.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub(crate) struct CalendarQuery {
    /// The first day, as `YYYY-MM-DD`.
    #[param(value_type = String, format = Date, example = "2026-11-01")]
    from: NaiveDate,
    /// The last day, as `YYYY-MM-DD`, at most 92 days after `from`.
    #[param(value_type = String, format = Date, example = "2026-11-30")]
    to: NaiveDate,
}

/// One post on the calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct CalendarPost {
    /// The post's identity, as in `/posts/{id}/`.
    id: String,
    /// The title.
    title: String,
    /// Draft, scheduled or published.
    status: PostStatus,
    /// Draft, in review or ready.
    review_state: ReviewState,
    /// When it went or goes live; empty for a draft, which sits on the day
    /// its `date` setting names.
    at: Option<DateTime<Utc>>,
}

impl CalendarPost {
    fn of(post: &AdminPost, at: Option<DateTime<Utc>>) -> Self {
        Self {
            id: post.id().to_owned(),
            title: post.title().to_owned(),
            status: post.status(),
            review_state: post.settings().review_state,
            at,
        }
    }
}

/// The posts on one day, in the order they go live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct CalendarDay {
    /// The day, in UTC.
    #[schema(value_type = String, format = Date)]
    date: NaiveDate,
    /// Its posts.
    posts: Vec<CalendarPost>,
}

/// Drafts, scheduled posts and published posts by day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Calendar {
    /// The first day.
    #[schema(value_type = String, format = Date)]
    from: NaiveDate,
    /// The last day.
    #[schema(value_type = String, format = Date)]
    to: NaiveDate,
    /// Every day with a post, earliest first. Days with none are left out.
    days: Vec<CalendarDay>,
    /// Drafts with no date and no schedule, to drag onto a day.
    undated: Vec<CalendarPost>,
}

/// The calendar as the Admin API wraps it.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CalendarBody {
    /// The calendar.
    calendar: Calendar,
}

impl PublishDoor {
    /// The posts from `from` to `to`, both included, by day in UTC: each
    /// published post on the day it went live, each scheduled post on the
    /// day it goes live, each other draft on its `date`, and drafts with
    /// neither in `undated`.
    ///
    /// # Errors
    ///
    /// Returns [`PostStoreError`] when the posts or their schedules cannot
    /// be read.
    pub async fn calendar(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Calendar, PostStoreError> {
        let mut days: BTreeMap<NaiveDate, Vec<CalendarPost>> = BTreeMap::new();
        let mut undated = Vec::new();
        let after = to.checked_add_days(Days::new(1)).unwrap_or(to);
        let live = format!("status:published+published_at:>='{from}'+published_at:<'{after}'");
        for id in self
            .ids(&live)
            .await?
            .into_iter()
            .chain(self.ids("status:draft").await?)
        {
            let Some(post) = self.get(&id).await? else {
                continue;
            };
            let at = post
                .published_at()
                .filter(|_| post.status() != PostStatus::Draft);
            let entry = CalendarPost::of(&post, at);
            match at.map(|at| at.date_naive()).or(post.settings().date) {
                Some(day) if (from..=to).contains(&day) => days.entry(day).or_default().push(entry),
                Some(_) => {}
                None => undated.push(entry),
            }
        }
        for posts in days.values_mut() {
            posts.sort_by(|a, b| (a.at, &a.id).cmp(&(b.at, &b.id)));
        }
        let days = days
            .into_iter()
            .map(|(date, posts)| CalendarDay { date, posts })
            .collect();
        Ok(Calendar {
            from,
            to,
            days,
            undated,
        })
    }

    /// The identity of every post `filter` picks, a page at a time.
    async fn ids(&self, filter: &str) -> Result<Vec<String>, PostStoreError> {
        let mut ids = Vec::new();
        for page in 1.. {
            let raw = RawListQuery {
                filter: Some(filter.to_owned()),
                limit: Some(PAGE.to_string()),
                page: Some(page.to_string()),
                ..RawListQuery::default()
            };
            let query = ListQuery::try_from(&raw).map_err(PostStoreError::new)?;
            let listed = self.list(&query).await?;
            let total = listed.total();
            let rows = listed.into_items();
            let empty = rows.is_empty();
            ids.extend(rows.iter().map(|row| row.id().to_owned()));
            if empty || u64::try_from(ids.len()).unwrap_or(u64::MAX) >= total {
                break;
            }
        }
        Ok(ids)
    }
}

/// The 400 for days that are missing, unreadable, backwards or too many.
fn bad_range() -> Problem {
    let message = UserMessage::new(
        "The calendar needs a first and a last day, at most 92 days apart.",
        "Each day is read from the posts, so a request covers a quarter at most.",
        "Send from and to as YYYY-MM-DD, such as from=2026-11-01&to=2026-11-30.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("from")
}

/// Lists drafts, scheduled posts and published posts by day from `from`
/// to `to`, at most 92 days, for the admin's calendar. Drafts with no date
/// and no schedule come under `undated`. To move a post to another day,
/// edit it: a new `date` for a draft, or `scheduled` with a new
/// `published_at`. Needs `posts:write`.
#[utoipa::path(
    get, path = "/calendar/", tag = "admin posts",
    security(("admin_key" = [])),
    params(CalendarQuery),
    responses(
        (status = 200, description = "The posts by day", body = CalendarBody),
        (status = 400, description = "from or to is missing or not a date, from is after to, or they are more than 92 days apart", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks posts:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn read_calendar(
    State(door): State<Arc<PublishDoor>>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<CalendarQuery>, QueryRejection>,
) -> Result<Json<CalendarBody>, Problem> {
    require(&principal, Scope::PostsWrite)?;
    let Query(CalendarQuery { from, to }) = query.map_err(|_| bad_range())?;
    let span = to.signed_duration_since(from).num_days();
    if !(0..=MAX_CALENDAR_DAYS).contains(&span) {
        return Err(bad_range());
    }
    let calendar = door
        .calendar(from, to)
        .await
        .map_err(|error| Problem::internal(&error))?;
    Ok(Json(CalendarBody { calendar }))
}
