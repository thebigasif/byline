//! Server analytics on the running site (KH-GRO-4): a layer over the pages
//! that counts each reader's view by where they came from and logs each
//! bot's request, with no cookie and nothing kept about the reader.
//!
//! The layer only notes what it saw. One task counts the notes into a
//! [`Tally`] and writes it to `data/site.db` every few seconds, so a
//! request never waits for the database.

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, Response, StatusCode, header};
use axum::middleware::{self, Next};
use khalid_analytics::{AnalyticsStore, Crawler, ReferrerGroup, Tally};
use khalid_core::{Clock, Config, SystemClock};
use khalid_db::{AnalyticsTable, Database, SiteId};
use khalid_outbound::Outbound;
use percent_encoding::percent_decode_str;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::redirect_route::asked_path;
use crate::visitor_log;

/// The most notes waiting to be counted. More are dropped, so a burst
/// cannot pile up in memory.
const WAITING: usize = 1024;

/// How often the tally is written.
const WRITE_EVERY: Duration = Duration::from_secs(5);

/// The longest address counted, in bytes. Longer ones are bot noise.
const LONGEST: usize = 512;

/// What the layer saw of one request.
#[derive(Debug)]
enum Note {
    /// A reader's view of a page, from a referrer group.
    View(String, ReferrerGroup),
    /// A bot's request for an address, with the status it was answered.
    Crawl(String, Crawler, u16),
}

/// What the layer holds: where to hand notes, and the mount folder.
type Counting = Arc<(mpsc::Sender<Note>, String)>;

/// The task that counts and writes, and the visitor log when it is on,
/// until [`Counter::stop`].
#[derive(Debug)]
pub(crate) struct Counter {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
    visitors: Option<visitor_log::Logger>,
}

/// The site's analytics in `db`, and `router` with each request under the
/// mount counted, and logged when `config` switches the visitor log on.
pub(crate) fn start(
    (db, site): (&Database, SiteId),
    router: Router,
    (folder, config, outbound): (&Path, &Config, &Outbound),
) -> (Counter, Router) {
    let store: Arc<dyn AnalyticsStore> = Arc::new(AnalyticsTable::new(db.clone(), site));
    let (mut counter, notes) = Counter::start(store, Arc::new(SystemClock));
    let counting: Counting = Arc::new((notes, config.mount().to_owned()));
    let router = router.layer(middleware::from_fn_with_state(counting, count));
    let (visitors, router) = visitor_log::start((folder, config, outbound), (db, site), router);
    counter.visitors = visitors;
    (counter, router)
}

impl Counter {
    /// Starts the task that counts each note at the time `clock` gives and
    /// writes the tally to `store`.
    fn start(store: Arc<dyn AnalyticsStore>, clock: Arc<dyn Clock>) -> (Self, mpsc::Sender<Note>) {
        let (sender, mut notes) = mpsc::channel(WAITING);
        let (stop, mut stopping) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut tally = Tally::default();
            let mut every = tokio::time::interval(WRITE_EVERY);
            loop {
                tokio::select! {
                    note = notes.recv() => match note {
                        Some(note) => add(&mut tally, note, clock.as_ref()),
                        None => break,
                    },
                    _ = every.tick() => write(store.as_ref(), &mut tally).await,
                    _ = &mut stopping => break,
                }
            }
            while let Ok(note) = notes.try_recv() {
                add(&mut tally, note, clock.as_ref());
            }
            write(store.as_ref(), &mut tally).await;
        });
        let visitors = None;
        (
            Self {
                stop,
                task,
                visitors,
            },
            sender,
        )
    }

    /// Writes what is left of the tally and the visitor log, and stops
    /// their tasks.
    pub(crate) async fn stop(self) {
        if let Some(visitors) = self.visitors {
            visitors.stop().await;
        }
        // The task may have ended already, when every sender is gone.
        self.stop.send(()).ok();
        if let Err(error) = self.task.await {
            tracing::warn!(%error, "the analytics task stopped early");
        }
    }
}

fn add(tally: &mut Tally, note: Note, clock: &dyn Clock) {
    match note {
        Note::View(path, group) => tally.view(clock.now(), &path, group),
        Note::Crawl(path, crawler, status) => tally.crawl(clock.now(), &path, crawler, status),
    }
}

async fn write(store: &dyn AnalyticsStore, tally: &mut Tally) {
    if tally.is_empty() {
        return;
    }
    let full = std::mem::take(tally);
    if let Err(error) = store.save(&full).await {
        tracing::warn!(%error, "the analytics were not written");
    }
}

/// Who asked, before the answer is known.
enum Seen {
    /// A bot, logged whatever the answer.
    Bot(String, Crawler),
    /// A reader, counted only when the answer is a page.
    Reader(String, ReferrerGroup),
}

/// The page's own answer, noted when it was a `GET` or `HEAD` under the
/// mount folder: a bot's request whatever its status, a reader's `GET`
/// only when it was a page.
async fn count(State(counting): State<Counting>, request: Request, next: Next) -> Response<Body> {
    let seen = seen(&counting.1, &request);
    let response = next.run(request).await;
    let note = match seen {
        Some(Seen::Bot(path, bot)) => Note::Crawl(path, bot, response.status().as_u16()),
        Some(Seen::Reader(path, group)) if is_page(&response) => Note::View(path, group),
        _ => return response,
    };
    if counting.0.try_send(note).is_err() {
        tracing::debug!("a view was dropped while the analytics were busy");
    }
    response
}

/// Who sent `request`, when it is one to note: a bot's `GET` or `HEAD`, or
/// a reader's `GET` that is not a browser fetching ahead.
fn seen(mount: &str, request: &Request) -> Option<Seen> {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return None;
    }
    let path = asked_path(mount, request)?;
    let path = path.as_str();
    if path.len() > LONGEST {
        return None;
    }
    let headers = request.headers();
    if let Some(bot) = Crawler::recognise(text(headers, header::USER_AGENT)) {
        return Some(Seen::Bot(path.to_owned(), bot));
    }
    if *request.method() != Method::GET || prefetch(headers) {
        return None;
    }
    let source = utm_source(request.uri().query());
    let (host, referer) = (text(headers, header::HOST), text(headers, header::REFERER));
    let group = ReferrerGroup::of(referer, host, source.as_deref());
    Some(Seen::Reader(path.to_owned(), group))
}

/// Whether `response` is a page a reader sees: a 200 with HTML.
fn is_page(response: &Response<Body>) -> bool {
    response.status() == StatusCode::OK
        && text(response.headers(), header::CONTENT_TYPE)
            .is_some_and(|kind| kind.starts_with("text/html"))
}

/// Whether the browser is fetching ahead, before the reader asked.
fn prefetch(headers: &HeaderMap) -> bool {
    ["sec-purpose", "purpose", "x-moz"].iter().any(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("prefetch") || value.contains("prerender"))
    })
}

/// The header `name` as text, when it is.
pub(crate) fn text(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// The `utm_source` in `query`, percent-encoding undone.
pub(crate) fn utm_source(query: Option<&str>) -> Option<String> {
    query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("utm_source="))
        .and_then(|value| percent_decode_str(value).decode_utf8().ok())
        .map(Cow::into_owned)
}
