//! Readers' comments on a running site (task 5.4): the comments each post
//! page shows, the form's route, and the Admin API's moderation, wired
//! from `khalid-comments` to the site database and the pages in memory.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use axum::Router;
use axum::response::Response;
use khalid_api::comment_moderation::{CommentModeration, NoComments};
use khalid_comments::{
    ApiModeration, Comment, CommentAuthor, CommentEvents, CommentStatus, CommentStore, Comments,
    FormRoute, PostFile, PostPage, PostPages, RateLimits, RefusalPages, Thread, form_router,
    threads,
};
use khalid_core::{Config, Mode, SystemClock};
use khalid_db::{CommentTable, Database, SiteId};
use khalid_render::{CommentWriter, PageBody, RenderComment, RenderComments, RenderThread};
use khalid_seo::SeoComment;
use khalid_server::{LivePages, Render, SecurityHeaders};

use crate::build_pages::LanguagePages;
use crate::site_pages::SitePages;

/// The approved comments each post page shows, by post file, and the post
/// pages that take comments, by address. Cloning shares them.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShownComments(Arc<Shown>);

#[derive(Debug, Default)]
struct Shown {
    threads: RwLock<BTreeMap<PostFile, Vec<Thread>>>,
    pages: RwLock<BTreeMap<String, PostPage>>,
}

impl ShownComments {
    /// Shows `all`, every post's threads, in place of those shown before.
    fn show_all(&self, all: BTreeMap<PostFile, Vec<Thread>>) {
        let mut shown = self
            .0
            .threads
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        *shown = all;
    }

    /// Shows `threads` on `post` in place of those shown before.
    fn show_on(&self, post: PostFile, threads: Vec<Thread>) {
        let mut shown = self
            .0
            .threads
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        shown.insert(post, threads);
    }
}

impl PostPages for ShownComments {
    fn post_at(&self, page: &str) -> Option<PostPage> {
        let pages = self.0.pages.read().unwrap_or_else(PoisonError::into_inner);
        pages.get(page).cloned()
    }
}

/// Gives each post page in `languages` its comments and the form under it,
/// and its schema the same comments, then notes the page as one that takes
/// comments.
/// A site that takes no comments, `shown` being `None`, is left as it is.
pub(crate) fn fill(
    config: &Config,
    shown: Option<&ShownComments>,
    languages: &mut [LanguagePages],
) {
    let Some(shown) = shown else {
        return;
    };
    let action = format!("{}comments/", config.mount());
    let threads = shown
        .0
        .threads
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    let mut pages = shown
        .0
        .pages
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    for language in languages {
        for page in &mut language.pages {
            let (PageBody::Post(post), Some(source)) = (&mut page.body, &page.source) else {
                continue;
            };
            let file = PostFile::new(source.to_string_lossy().replace('\\', "/"));
            let on_post = threads.get(&file).map_or(&[][..], Vec::as_slice);
            post.comments = Some(RenderComments {
                threads: on_post.iter().map(render_thread).collect(),
                action: action.clone(),
                page: page.meta.path.clone(),
            });
            page.seo.comments = on_post.iter().map(seo_comment).collect();
            let language = language.site.locale.clone();
            let href = page.meta.path.clone();
            pages.insert(
                href.clone(),
                PostPage {
                    file,
                    href,
                    language,
                },
            );
        }
    }
}

fn render_comment(comment: &Comment) -> RenderComment {
    RenderComment {
        id: comment.id.get(),
        author: comment.author.name().as_str().to_owned(),
        writer: match comment.author {
            CommentAuthor::Reader { .. } => CommentWriter::Reader,
            CommentAuthor::Staff { .. } => CommentWriter::Staff,
        },
        date: comment.created_at.date_naive(),
        paragraphs: RenderComment::paragraphs_of(comment.text.as_str()),
    }
}

fn render_thread(thread: &Thread) -> RenderThread {
    RenderThread {
        comment: render_comment(&thread.comment),
        replies: thread.replies.iter().map(render_comment).collect(),
    }
}

fn seo_comment(thread: &Thread) -> SeoComment {
    let one = |comment: &Comment| SeoComment {
        id: comment.id.get(),
        author: comment.author.name().as_str().to_owned(),
        text: comment.text.as_str().to_owned(),
        published: comment.created_at,
        replies: Vec::new(),
    };
    SeoComment {
        replies: thread.replies.iter().map(one).collect(),
        ..one(&thread.comment)
    }
}

/// Renders a post page again when the comments it shows change: with the
/// approved comments read from `store`, alone when it can be, else with
/// the whole site.
#[derive(Debug)]
struct PageEvents {
    store: CommentTable,
    shown: ShownComments,
    source: SitePages,
    live: LivePages,
}

impl CommentEvents for PageEvents {
    fn shown_changed(&self, post: &PostFile) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let (store, shown, post) = (self.store.clone(), self.shown.clone(), post.clone());
        let (source, live) = (self.source.clone(), self.live.clone());
        runtime.spawn(async move {
            let approved = store.with_status(CommentStatus::Approved, Some(&post));
            match approved.await {
                Ok(approved) => shown.show_on(post.clone(), threads(approved)),
                Err(error) => return tracing::warn!(%error, "comments could not be read"),
            }
            let alone = source.clone();
            let file = PathBuf::from(post.as_str());
            let saved = tokio::task::spawn_blocking(move || alone.render_saved(&[file])).await;
            match saved {
                Ok(Ok(Some(pages))) => live.show_saved(pages),
                _ => {
                    if let Err(error) = live.refresh(&source).await {
                        tracing::warn!(%error, "a post page could not show its comments");
                    }
                }
            }
        });
    }
}

/// The comments of a running site: the routes of the reader form and the
/// Admin API's moderation. A site not in server mode takes no comments:
/// no form route, and moderation finds nothing.
pub(crate) async fn start(
    config: &Config,
    (db, site): (&Database, SiteId),
    (source, live): (&SitePages, &LivePages),
    security: SecurityHeaders,
) -> (Router, Arc<dyn CommentModeration>) {
    let (Some(shown), Some(stylesheet)) = (source.comments(), source.stylesheet()) else {
        return (Router::new(), Arc::new(NoComments));
    };
    let store = CommentTable::new(db.clone(), site);
    let events = PageEvents {
        store: store.clone(),
        shown: shown.clone(),
        source: source.clone(),
        live: live.clone(),
    };
    let comments = Comments::new(store, SystemClock).with_events(Arc::new(events));
    match comments.shown().await {
        Ok(all) => shown.show_all(all),
        Err(error) => tracing::warn!(%error, "approved comments could not be read"),
    }
    let comments = Arc::new(comments);
    let Ok(refusals) = RefusalPages::load() else {
        return (Router::new(), Arc::new(ApiModeration::new(comments)));
    };
    let route = FormRoute {
        comments: Arc::clone(&comments),
        limits: RateLimits::new(SystemClock),
        pages: Arc::new(shown),
        refusals,
        action: format!("{}comments/", config.mount()),
        stylesheet,
        language: config.default_language().clone(),
    };
    let secured = move |mut response: Response| {
        security.apply(response.headers_mut());
        async move { response }
    };
    let form = form_router(route).layer(axum::middleware::map_response(secured));
    (form, Arc::new(ApiModeration::new(comments)))
}

/// Whether a site in `mode` takes comments: only a site Khalid serves.
pub(crate) fn taken(mode: Mode) -> bool {
    mode == Mode::Server
}
