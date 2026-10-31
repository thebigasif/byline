//! The visitor log on the running site (GEO C1): while `[visitor_log]` is
//! on, a layer notes each `GET` and `HEAD` under the mount folder, and one
//! task turns the notes into [`Visit`] rows and writes them to
//! `data/site.db` every few seconds. Off, it adds nothing.
//!
//! The full address and user agent stay in memory: the task keeps only a
//! [`ShortAddress`], a [`VisitorCode`] made with a key it replaces at
//! midnight UTC, and for a bot whether its address is on the list its
//! company publishes. `docs/design/visitor-log.md` is the design.

use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, Method, Response, header};
use axum::middleware::{self, Next};
use chrono::{DateTime, Days, NaiveDate, Utc};
use khalid_analytics::{
    Crawler, CrawlerRanges, MAX_VISITS_PER_DAY, ReferrerGroup, ShortAddress, Visit, Visitor,
    VisitorCode, VisitorKey, VisitorLogStore,
};
use khalid_core::{Clock, Config, SystemClock};
use khalid_db::{Database, SiteId, VisitorLogTable};
use khalid_outbound::Outbound;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::crawler_lists;
use crate::page_counts::{text, utm_source};
use crate::redirect_route::asked_path;

/// The most notes waiting to be logged. More are dropped, so a burst
/// cannot pile up in memory.
const WAITING: usize = 1024;

/// How often the log is written.
const WRITE_EVERY: Duration = Duration::from_secs(5);

/// The longest address logged, in bytes. Longer ones are bot noise.
const LONGEST: usize = 512;

/// What the layer saw of one request.
#[derive(Debug)]
struct Note {
    path: String,
    status: u16,
    address: IpAddr,
    user_agent: Option<String>,
    who: Who,
}

#[derive(Debug)]
enum Who {
    Bot(Crawler),
    Person(ReferrerGroup),
}

/// What the layer holds: where to hand notes, and the mount folder.
type Logging = Arc<(mpsc::Sender<Note>, String)>;

/// The tasks that log visits and fetch the address lists, until
/// [`Logger::stop`].
#[derive(Debug)]
pub(crate) struct Logger {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
    fetching: JoinHandle<()>,
}

/// `router` with each visit under the site's mount logged in `db`, and
/// the tasks doing it, when `config` switches the visitor log on; else
/// `router` as it was. The address lists are kept under the `site`
/// folder and fetched through `outbound`.
pub(crate) fn start(
    (site, config, outbound): (&Path, &Config, &Outbound),
    (db, id): (&Database, SiteId),
    router: Router,
) -> (Option<Logger>, Router) {
    let settings = config.visitor_log();
    if !settings.on {
        return (None, router);
    }
    let (fetching, reading) = crawler_lists::start(site, outbound);
    let store: Arc<dyn VisitorLogStore> = Arc::new(VisitorLogTable::new(db.clone(), id));
    let (sender, notes) = mpsc::channel(WAITING);
    let (stop, stopping) = oneshot::channel();
    let kept = Days::new(u64::from(settings.keep_days));
    let task = tokio::spawn(log(
        (store, Arc::new(SystemClock)),
        kept,
        reading,
        (notes, stopping),
    ));
    let logging: Logging = Arc::new((sender, config.mount().to_owned()));
    let logger = Logger {
        stop,
        task,
        fetching,
    };
    (
        Some(logger),
        router.layer(middleware::from_fn_with_state(logging, note)),
    )
}

impl Logger {
    /// Writes the visits not yet written and stops both tasks.
    pub(crate) async fn stop(self) {
        self.fetching.abort();
        // The task may have ended already, when every sender is gone.
        self.stop.send(()).ok();
        if let Err(error) = self.task.await {
            tracing::warn!(%error, "the visitor log task stopped early");
        }
    }
}

/// The day's key, made again when the day changes.
struct DailyKey {
    day: NaiveDate,
    key: Option<VisitorKey>,
}

impl DailyKey {
    /// The code of the visitor at `address` with `user_agent` at `now`, or
    /// `None` when no random key could be made.
    fn code(
        &mut self,
        now: DateTime<Utc>,
        address: IpAddr,
        agent: Option<&str>,
    ) -> Option<VisitorCode> {
        if self.key.is_none() || self.day != now.date_naive() {
            self.day = now.date_naive();
            self.key = new_key();
        }
        self.key.as_ref().map(|key| key.code(address, agent))
    }
}

fn new_key() -> Option<VisitorKey> {
    let mut bytes = [0; 32];
    match getrandom::fill(&mut bytes) {
        Ok(()) => Some(VisitorKey::from_bytes(bytes)),
        Err(error) => {
            tracing::warn!(%error, "no random key for visitor codes, so visits are not logged");
            None
        }
    }
}

/// Logs each note at the time `clock` gives, writing to `store` every few
/// seconds and erasing what is older than `kept` days at least daily.
async fn log(
    (store, clock): (Arc<dyn VisitorLogStore>, Arc<dyn Clock>),
    kept: Days,
    ranges: watch::Receiver<CrawlerRanges>,
    (mut notes, mut stopping): (mpsc::Receiver<Note>, oneshot::Receiver<()>),
) {
    let mut key = DailyKey {
        day: clock.now().date_naive(),
        key: None,
    };
    let mut visits = Vec::new();
    let mut erased = None;
    let mut every = tokio::time::interval(WRITE_EVERY);
    loop {
        tokio::select! {
            note = notes.recv() => match note {
                // A day keeps no more rows than this, so a flood stops here.
                Some(_) if visits.len() >= MAX_VISITS_PER_DAY => {}
                Some(note) => visits.extend(visit(note, clock.now(), &mut key, &ranges.borrow())),
                None => break,
            },
            _ = every.tick() => write(store.as_ref(), (clock.now(), kept), &mut visits, &mut erased).await,
            _ = &mut stopping => break,
        }
    }
    while let Ok(note) = notes.try_recv() {
        if visits.len() >= MAX_VISITS_PER_DAY {
            break;
        }
        visits.extend(visit(note, clock.now(), &mut key, &ranges.borrow()));
    }
    write(
        store.as_ref(),
        (clock.now(), kept),
        &mut visits,
        &mut erased,
    )
    .await;
}

/// The row `note` makes at `now`, with nothing of the full address or
/// user agent left in it.
fn visit(
    note: Note,
    now: DateTime<Utc>,
    key: &mut DailyKey,
    ranges: &CrawlerRanges,
) -> Option<Visit> {
    let code = key.code(now, note.address, note.user_agent.as_deref())?;
    let visitor = match note.who {
        Who::Bot(bot) => Visitor::Bot(bot, ranges.check(bot, note.address)),
        Who::Person(group) => Visitor::Person(group),
    };
    Some(Visit {
        at: now,
        path: note.path,
        status: note.status,
        address: ShortAddress::of(note.address),
        code,
        visitor,
    })
}

/// Writes `visits`, or erases the old ones when nothing has been erased
/// today, keeping the `kept` days up to `now`.
async fn write(
    store: &dyn VisitorLogStore,
    (now, kept): (DateTime<Utc>, Days),
    visits: &mut Vec<Visit>,
    erased: &mut Option<NaiveDate>,
) {
    let today = now.date_naive();
    if visits.is_empty() && *erased == Some(today) {
        return;
    }
    let keep_from = now.checked_sub_days(kept).unwrap_or(now);
    let written = std::mem::take(visits);
    match store.save(&written, keep_from).await {
        Ok(()) => *erased = Some(today),
        Err(error) => tracing::warn!(%error, "the visitor log was not written"),
    }
}

/// Notes the request after it is answered, when it was a `GET` or `HEAD`
/// under the mount folder from an address the server can name.
async fn note(State(logging): State<Logging>, request: Request, next: Next) -> Response<Body> {
    let seen = seen(&logging.1, &request);
    let response = next.run(request).await;
    if let Some((path, address, user_agent, who)) = seen {
        let status = response.status().as_u16();
        let note = Note {
            path,
            status,
            address,
            user_agent,
            who,
        };
        if logging.0.try_send(note).is_err() {
            tracing::debug!("a visit was dropped while the visitor log was busy");
        }
    }
    response
}

/// What a note needs from `request`, before it is answered.
fn seen(mount: &str, request: &Request) -> Option<(String, IpAddr, Option<String>, Who)> {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return None;
    }
    let path = asked_path(mount, request)?;
    let path = path.as_str();
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()?
        .0
        .ip();
    if path.len() > LONGEST {
        return None;
    }
    let headers = request.headers();
    let agent = text(headers, header::USER_AGENT);
    let who = Crawler::recognise(agent).map_or_else(
        || {
            let source = utm_source(request.uri().query());
            let (host, referer) = (text(headers, header::HOST), text(headers, header::REFERER));
            Who::Person(ReferrerGroup::of(referer, host, source.as_deref()))
        },
        Who::Bot,
    );
    let address = visitor_address(peer, headers);
    Some((path.to_owned(), address, agent.map(str::to_owned), who))
}

/// The visitor's address: the peer's, or, when the peer is a proxy on
/// this machine or the local network, the last address its
/// `X-Forwarded-For` names, which is the one the proxy saw.
fn visitor_address(peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    let local = match peer.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    };
    if !local {
        return peer;
    }
    let forwarded = headers.get_all("x-forwarded-for").into_iter().next_back();
    forwarded
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|last| last.trim().parse().ok())
        .unwrap_or(peer)
}
