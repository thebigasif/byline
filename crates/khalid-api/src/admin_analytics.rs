//! Analytics through the Admin API: the page views per day, source and
//! page (KH-GRO-4), and AI visitors per tool (GEO M1): how many readers
//! each AI assistant sent, against the period before. The routes need
//! `settings:write`.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::{Extension, Json, middleware};
use chrono::NaiveDate;
use khalid_analytics::{Period, ToolVisitors};
use khalid_core::UserMessage;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::admin_key::{AdminDoor, require, require_admin_key};
use crate::analytics_desk::AnalyticsDesk;
use crate::api_version::negotiate_version;
use crate::keys::{Principal, Scope};
use crate::problem::{Problem, ProblemType};

/// How many days a report covers when none are asked for.
const DEFAULT_DAYS: u32 = 28;

/// The analytics routes, behind an integration key or staff token.
pub(crate) fn routes(desk: AnalyticsDesk, door: AdminDoor) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ai_visitors))
        .routes(routes!(visitors))
        .route_layer(middleware::from_fn_with_state(door, require_admin_key))
        .route_layer(middleware::from_fn(negotiate_version))
        .with_state(desk)
}

/// How many days to count, ending today.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct VisitorsQuery {
    /// From 1 to 366; 28 when left out.
    days: Option<u32>,
}

/// The views of one page from one assistant.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminPageViews {
    /// The page's path.
    #[schema(example = "/blog/internal-links/")]
    path: String,
    /// How many views.
    views: u64,
}

/// What one AI assistant sent.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminToolVisitors {
    /// The assistant, for programs: `chatgpt`, `perplexity`, `gemini`,
    /// `copilot`, `claude`, `deepseek`, `le-chat`, `meta-ai`, `grok`,
    /// `you`, `phind`, or `unnamed` for views counted before tools were
    /// named.
    #[schema(example = "chatgpt")]
    tool: &'static str,
    /// The assistant's name, for people.
    #[schema(example = "ChatGPT")]
    name: &'static str,
    /// Views from its readers in the period.
    views: u64,
    /// Views from its readers in the period of the same length before.
    before: u64,
    /// The pages its readers landed on, most views first, at most ten.
    pages: Vec<AdminPageViews>,
}

/// The readers AI assistants sent, per tool. The counts are page views,
/// not people, and never include AI bots.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminAiVisitors {
    /// The first day counted, in UTC.
    first: NaiveDate,
    /// The last day counted: today, in UTC.
    last: NaiveDate,
    /// How many days were counted.
    days: u32,
    /// Every view that came from outside the site, AI search included.
    arrivals: u64,
    /// The views AI assistants sent.
    ai_views: u64,
    /// Each assistant that sent readers in the period or the one before,
    /// most views first.
    tools: Vec<AdminToolVisitors>,
}

/// Lists how many readers each AI assistant sent in the last `days` days,
/// against the days before, with the pages they landed on. Needs
/// `settings:write`.
#[utoipa::path(
    get, path = "/analytics/ai-visitors/", tag = "admin analytics",
    security(("admin_key" = [])),
    params(VisitorsQuery),
    responses(
        (status = 200, description = "The readers per assistant", body = AdminAiVisitors),
        (status = 400, description = "The days are not from 1 to 366", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn ai_visitors(
    State(desk): State<AnalyticsDesk>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<VisitorsQuery>, QueryRejection>,
) -> Result<Json<AdminAiVisitors>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let period = period(&desk, query)?;
    let report = desk
        .ai_visitors(period)
        .await
        .map_err(|e| Problem::internal(&e))?;
    Ok(Json(AdminAiVisitors {
        first: period.first(),
        last: period.last(),
        days: period.days(),
        arrivals: report.arrivals,
        ai_views: report.ai_views,
        tools: report.tools.into_iter().map(tool).collect(),
    }))
}

fn tool(row: ToolVisitors) -> AdminToolVisitors {
    let pages = row.pages.into_iter().map(|page| AdminPageViews {
        path: page.path,
        views: page.views,
    });
    AdminToolVisitors {
        tool: row.tool.key(),
        name: row.tool.name(),
        views: row.views,
        before: row.before,
        pages: pages.collect(),
    }
}

/// The views from one source.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminSourceViews {
    /// Where they came from: `direct`, `search`, `social`, `ai` (every AI
    /// assistant), `other` (any other site) or `internal` (the site's own
    /// pages).
    #[schema(example = "search")]
    source: &'static str,
    /// Views in the period.
    views: u64,
    /// Views in the period of the same length before.
    before: u64,
}

/// The views on one day.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminDayViews {
    /// The day, in UTC.
    day: NaiveDate,
    /// Views that day.
    views: u64,
}

/// The page views `khalid serve` counted. A view is one page opened by a
/// person; bots never count.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AdminVisitors {
    /// The first day counted, in UTC.
    first: NaiveDate,
    /// The last day counted: today, in UTC.
    last: NaiveDate,
    /// How many days were counted.
    days: u32,
    /// Every view in the period.
    views: u64,
    /// Every view in the period of the same length before.
    before: u64,
    /// The views that came from outside the site.
    arrivals: u64,
    /// The views per source, every source listed, outside ones first.
    sources: Vec<AdminSourceViews>,
    /// The views per day, first day first, every day listed.
    per_day: Vec<AdminDayViews>,
    /// The most viewed pages, most views first, at most twenty.
    pages: Vec<AdminPageViews>,
}

/// Counts the page views of the last `days` days, per day, source and
/// page, against the days before. Needs `settings:write`.
#[utoipa::path(
    get, path = "/analytics/visitors/", tag = "admin analytics",
    security(("admin_key" = [])),
    params(VisitorsQuery),
    responses(
        (status = 200, description = "The page views", body = AdminVisitors),
        (status = 400, description = "The days are not from 1 to 366", body = Problem, content_type = "application/problem+json"),
        (status = 401, description = "No valid integration key or staff token", body = Problem, content_type = "application/problem+json"),
        (status = 403, description = "The key lacks settings:write", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn visitors(
    State(desk): State<AnalyticsDesk>,
    Extension(principal): Extension<Principal>,
    query: Result<Query<VisitorsQuery>, QueryRejection>,
) -> Result<Json<AdminVisitors>, Problem> {
    require(&principal, Scope::SettingsWrite)?;
    let period = period(&desk, query)?;
    let counts = desk
        .visitors(period)
        .await
        .map_err(|e| Problem::internal(&e))?;
    let sources = counts.sources.iter().map(|row| AdminSourceViews {
        source: row.source.key(),
        views: row.views,
        before: row.before,
    });
    let per_day = counts.days.iter().map(|row| AdminDayViews {
        day: row.day,
        views: row.views,
    });
    let pages = counts.pages.into_iter().map(|page| AdminPageViews {
        path: page.path,
        views: page.views,
    });
    Ok(Json(AdminVisitors {
        first: period.first(),
        last: period.last(),
        days: period.days(),
        views: counts.views,
        before: counts.before,
        arrivals: counts.arrivals,
        sources: sources.collect(),
        per_day: per_day.collect(),
        pages: pages.collect(),
    }))
}

/// The period the query asks for, ending today.
fn period(
    desk: &AnalyticsDesk,
    query: Result<Query<VisitorsQuery>, QueryRejection>,
) -> Result<Period, Problem> {
    let days = query.ok().map(|Query(q)| q.days.unwrap_or(DEFAULT_DAYS));
    days.and_then(|days| Period::ending(desk.today(), days))
        .ok_or_else(bad_days)
}

/// The 400 for days that are not from 1 to 366.
fn bad_days() -> Problem {
    let message = UserMessage::new(
        "The days are not a number from 1 to 366.",
        "The report counts back from today, at most a year.",
        "Ask for days=28, or leave it out.",
    );
    Problem::new(ProblemType::InvalidQuery, &message).with_rule("days")
}
