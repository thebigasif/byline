//! The toolbar on public pages for signed-in staff (plan section 4.6,
//! KH-EDT-6): on a post, a link to edit it in the admin, what the linter
//! says about it and how often it was read in the last 30 days.
//!
//! The bar is added on the server, so it needs no script. Only a request
//! with a live staff session gets it, and that answer is marked private
//! and never stored, so no cache hands one person's bar to a reader. A
//! reader's request passes through untouched.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, ETAG, IF_NONE_MATCH, VARY,
};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use chrono::Duration;
use khalid_analytics::AnalyticsStore;
use khalid_api::content_store::{ContentStore, PostKey};
use khalid_api::post_store::PostStore;
use khalid_api::resources::{Collection, LintCounts};
use khalid_auth::{Accounts, SessionToken};
use khalid_core::{Clock, Config, Slug, SystemClock};
use khalid_db::{AnalyticsTable, Database, SiteId};

use crate::site_content::LiveContent;
use crate::site_posts::SitePosts;
use crate::staff;

/// The toolbar's styles. A file of its own, since the pages' policy allows
/// only stylesheets from the site, and only staff ever load it.
const STYLES: &str = include_str!("staff_toolbar.css");

/// The largest page the bar is added to.
const PAGE_LIMIT: usize = 4 * 1024 * 1024;

/// What the bar reads: who is signed in, the posts and their numbers.
struct Toolbar {
    accounts: Accounts,
    content: Arc<dyn ContentStore>,
    posts: SitePosts,
    views: Arc<dyn AnalyticsStore>,
    clock: Arc<dyn Clock>,
    /// The mount folder, such as `/blog/`.
    mount: String,
}

/// `router`, the site's pages, with the bar on each page staff of site
/// `id` ask for, and the bar's stylesheet at `<mount>_toolbar.css`.
pub(crate) fn start(
    (database, id, config): (&Database, SiteId, &Config),
    (content, posts): (LiveContent, SitePosts),
    router: Router,
) -> Router {
    let toolbar = Toolbar {
        accounts: staff::accounts(database, id, config),
        content: Arc::new(content),
        posts,
        views: Arc::new(AnalyticsTable::new(database.clone(), id)),
        clock: Arc::new(SystemClock),
        mount: config.mount().to_owned(),
    };
    router.layer(middleware::from_fn_with_state(Arc::new(toolbar), show))
}

async fn show(State(toolbar): State<Arc<Toolbar>>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    if path == toolbar.stylesheet() {
        let headers = [(CONTENT_TYPE, "text/css; charset=utf-8")];
        return (headers, STYLES).into_response();
    }
    let token = request
        .headers()
        .get(COOKIE)
        .and_then(|value| value.to_str().ok());
    let token = token.and_then(SessionToken::from_cookie_header);
    if request.method() != Method::GET || !toolbar.signed_in(token).await {
        return next.run(request).await;
    }
    // The bar is added to the page's own bytes, so ask for them plain and
    // whole: no compressed copy and no "not modified".
    request.headers_mut().remove(ACCEPT_ENCODING);
    request.headers_mut().remove(IF_NONE_MATCH);
    let response = next.run(request).await;
    let html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|media| media.starts_with("text/html"));
    if response.status() != StatusCode::OK || !html {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let Ok(page) = to_bytes(body, PAGE_LIMIT).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let page = String::from_utf8_lossy(&page);
    let bar = toolbar.bar(&path).await;
    let shown = match page.rfind("</body>") {
        Some(end) => {
            let (head, tail) = page.split_at(end);
            format!("{head}{bar}{tail}")
        }
        None => format!("{page}{bar}"),
    };
    parts.headers.remove(ETAG);
    parts.headers.remove(CONTENT_LENGTH);
    let private = HeaderValue::from_static("private, no-store");
    parts.headers.insert(CACHE_CONTROL, private);
    parts
        .headers
        .insert(VARY, HeaderValue::from_static("cookie"));
    Response::from_parts(parts, Body::from(shown))
}

impl Toolbar {
    fn stylesheet(&self) -> String {
        format!("{}_toolbar.css", self.mount)
    }

    /// Whether `token`, from the request's cookie, is a live staff session.
    async fn signed_in(&self, token: Option<SessionToken>) -> bool {
        match token {
            Some(token) => self.accounts.session(&token).await.is_ok(),
            None => false,
        }
    }

    /// The identity of the published post at `path`, as the Admin API
    /// names it, such as `en.hello`.
    async fn post_at(&self, path: &str) -> Option<String> {
        let last = path.trim_end_matches('/').rsplit('/').next()?;
        let slug = Slug::try_new_unicode(last).ok()?;
        let key = PostKey::Slug(slug);
        let found = self.content.find(Collection::Posts, &key);
        let post = found.await.ok().flatten()?;
        let here = post.url().as_str().ends_with(path);
        here.then(|| format!("{}.{}", post.language(), post.slug()))
    }

    /// The bar for the page at `path`.
    async fn bar(&self, path: &str) -> String {
        let admin = format!("{}admin", self.mount);
        let opening = format!(
            "<link rel=\"stylesheet\" href=\"{}\"><aside class=\"khalid-toolbar\" \
             aria-label=\"Staff toolbar\"><p>You are signed in. Readers do not see this bar.</p>",
            self.stylesheet()
        );
        let Some(id) = self.post_at(path).await else {
            return format!("{opening}<a href=\"{admin}/\">Open the admin</a></aside>");
        };
        let lint = match self.posts.lint(&id).await {
            Ok(Some(report)) => lint_words(LintCounts::of(&report)),
            _ => "The linter could not check this post just now.".to_owned(),
        };
        let views = self.views_of(path).await;
        let id = escaped(&id);
        format!(
            "{opening}<a href=\"{admin}/posts/{id}\">Edit this post</a>\
             <p>{lint}</p><p>{views}</p></aside>"
        )
    }

    /// How often the page at `path` was read in the last 30 days, in words.
    async fn views_of(&self, path: &str) -> String {
        let today = self.clock.now().date_naive();
        let first = today - Duration::days(29);
        match self.views.views(first, today).await {
            Ok(rows) => {
                let read: u64 = rows
                    .iter()
                    .filter(|r| r.path == path)
                    .map(|r| r.views)
                    .sum();
                let times = if read == 1 { "time" } else { "times" };
                format!("Read {read} {times} in the last 30 days.")
            }
            Err(_) => "The numbers could not be read just now.".to_owned(),
        }
    }
}

/// What the linter found, in words.
fn lint_words(counts: LintCounts) -> String {
    let warnings = match counts.warnings() {
        0 => String::new(),
        1 => ", with 1 warning".to_owned(),
        n => format!(", with {n} warnings"),
    };
    match counts.blocks() {
        0 => format!("Passes the lint check{warnings}."),
        1 => format!("1 problem would stop this post going live{warnings}."),
        n => format!("{n} problems would stop this post going live{warnings}."),
    }
}

/// `text` with the characters HTML gives meaning to escaped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lint_result_reads_as_a_sentence() {
        let said = |blocks, warnings| lint_words(LintCounts::new(blocks, warnings));
        assert_eq!(said(0, 0), "Passes the lint check.");
        assert_eq!(said(0, 1), "Passes the lint check, with 1 warning.");
        assert_eq!(said(1, 0), "1 problem would stop this post going live.");
        let both = "2 problems would stop this post going live, with 3 warnings.";
        assert_eq!(said(2, 3), both);
        assert_eq!(escaped("en.a<b>&\"'"), "en.a&lt;b&gt;&amp;&quot;&#39;");
    }
}
