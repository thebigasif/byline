//! The alert checks `khalid serve` runs once an hour (KH-GRO-6): a burst
//! of 404s against the last reading of the 404 log, the certificate in
//! `data/https/` when the server has one, and live posts left unchanged
//! too long. A scheduled post the linter refuses is raised by the
//! scheduler as it happens, and broken links by `khalid audit`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use khalid_alerts::{Kind, LivePost, ServedCertificate, certificate, not_found_spike, stale_posts};
use khalid_api::content_store::ContentStore;
use khalid_api::not_found_log::{MAX_MISSES, NotFoundLog};
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_api::resources::Collection;
use khalid_core::{AlertSettings, Config};
use khalid_db::{Database, SiteId};
use khalid_https::{Domain, HttpsFolder};
use khalid_newsletter::Mailer;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, Interval, MissedTickBehavior};

use crate::alerts::{self, Alerting};
use crate::announce::SiteAnnounce;
use crate::cli::ServeArgs;
use crate::site_content::LiveContent;
use crate::site_watch::{self, SiteWatch};

/// How often the checks run.
const EVERY: Duration = Duration::from_hours(1);

/// How often the checks run, and the speed and uptime watch looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Every {
    /// The alert checks.
    pub(crate) checks: Duration,
    /// The watch, or `None` when it does not look.
    pub(crate) watch: Option<Duration>,
}

/// How often the checks run for `args`: hourly, or as the hidden
/// `--alert-every` asks in a test build. The watch looks as
/// [`site_watch::every`] says for the site with `config`.
pub(crate) fn every(args: &ServeArgs, config: &Config) -> Every {
    let watch = site_watch::every(args, config);
    #[cfg(feature = "test-util")]
    if let Some(seconds) = args.alert_every {
        let checks = Duration::from_secs(seconds);
        return Every { checks, watch };
    }
    Every {
        checks: EVERY,
        watch,
    }
}

/// What the checks read.
pub(crate) struct Sources {
    /// The 404 log, read and never changed.
    pub(crate) misses: Arc<dyn NotFoundLog>,
    /// The published posts readers see now.
    pub(crate) content: LiveContent,
    /// The domain and the folder of the certificate, when serving HTTPS.
    pub(crate) https: Option<(Domain, HttpsFolder)>,
    /// `[alerts]` from `site.toml`.
    pub(crate) settings: AlertSettings,
    /// The speed and uptime watch and how often it looks, when it does.
    pub(crate) watch: Option<(SiteWatch, Duration)>,
}

/// Alerts made ready before the server listens, started once the site's
/// announcer is.
pub(crate) struct Ready {
    /// The one way mail leaves, shared with the newsletter.
    pub(crate) mailer: Option<Arc<dyn Mailer>>,
    /// What the checks read.
    pub(crate) sources: Sources,
    /// How often the checks run.
    pub(crate) every: Duration,
}

impl Ready {
    /// Starts alerting for the site with `config`, keeping raised alerts in
    /// site `id` of `database` and firing webhooks through `telling`, and
    /// the checks. Must be called inside the tokio runtime.
    pub(crate) fn start(
        self,
        (config, database, id, telling): (&Config, &Database, SiteId, SiteAnnounce),
    ) -> (Arc<Alerting>, Watching) {
        let alerting = alerts::for_site(config, (database, id), self.mailer, telling);
        let watching = start(Arc::clone(&alerting), self.sources, self.every);
        (alerting, watching)
    }
}

/// The running checks, stopped before the database closes.
#[derive(Debug)]
pub(crate) struct Watching {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl Watching {
    /// Stops the checks once the round in progress has finished
    /// (RULES.md BL-RUN-10).
    pub(crate) async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Reads the 404 log once now, so the first round can tell a burst, then
/// runs the checks every `every` through `alerting`, and the speed and
/// uptime watch as often as it looks. Must be called inside the tokio
/// runtime.
fn start(alerting: Arc<Alerting>, sources: Sources, every: Duration) -> Watching {
    let (stop, mut stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut before = counts(&*sources.misses).await;
        let mut ticks = ticking(every);
        let mut looks = sources.watch.as_ref().map(|(_, every)| ticking(*every));
        loop {
            tokio::select! {
                _ = &mut stopped => return,
                _ = ticks.tick() => before = round(&alerting, &sources, before).await,
                () = next(looks.as_mut()) => {
                    if let Some((watch, _)) = &sources.watch {
                        watch.round(&alerting).await;
                    }
                }
            }
        }
    });
    Watching { stop, task }
}

/// Ticks every `every`, the first after one wait; a late tick waits
/// rather than bunching up.
fn ticking(every: Duration) -> Interval {
    let mut ticks = tokio::time::interval_at(Instant::now() + every, every);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticks
}

/// The next tick of `ticks`, or never when there are none.
async fn next(ticks: Option<&mut Interval>) {
    match ticks {
        Some(ticks) => {
            ticks.tick().await;
        }
        None => std::future::pending().await,
    }
}

/// One round of every check. Returns the 404 counts read, for the next.
async fn round(
    alerting: &Alerting,
    sources: &Sources,
    before: Option<BTreeMap<String, u64>>,
) -> Option<BTreeMap<String, u64>> {
    let at = alerting.now();
    let now = counts(&*sources.misses).await;
    if let (Some(before), Some(now)) = (&before, &now) {
        let spike = sources.settings.not_found_spike;
        let found = not_found_spike(before, now, spike, at);
        alerting.raise(Kind::NotFoundSpike, found).await;
    }
    if let Some((domain, folder)) = &sources.https {
        match folder.certificate().await {
            Ok(Some(held)) => {
                let domain = domain.as_str().to_owned();
                let served = ServedCertificate {
                    domain,
                    not_after: held.not_after(),
                };
                alerting
                    .raise(Kind::Certificate, certificate(&served, at))
                    .await;
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "the certificate could not be read for its alert"),
        }
    }
    let days = sources.settings.stale_after_days;
    if days > 0 {
        let live = live_posts(&sources.content).await;
        alerting
            .raise(Kind::StalePost, stale_posts(&live, days, at))
            .await;
    }
    alerting.tidy().await;
    now.or(before)
}

/// Each address in the 404 log with its count, or `None` when the log
/// cannot be read, which is logged.
async fn counts(log: &dyn NotFoundLog) -> Option<BTreeMap<String, u64>> {
    let limit = u32::try_from(MAX_MISSES).unwrap_or(u32::MAX);
    match log.misses(limit).await {
        Ok(misses) => Some(
            misses
                .into_iter()
                .map(|miss| (miss.path, miss.hits))
                .collect(),
        ),
        Err(error) => {
            tracing::warn!(%error, "the 404 log could not be read for its alert");
            None
        }
    }
}

/// Every published post with when it last changed, as the Content API
/// shows it. A failure is logged and leaves the list empty.
async fn live_posts(content: &LiveContent) -> Vec<LivePost> {
    let mut raw = RawListQuery::default();
    raw.limit = Some("all".to_owned());
    let Ok(query) = ListQuery::try_from(&raw) else {
        return Vec::new();
    };
    let listed = match content.list(Collection::Posts, &query).await {
        Ok(listed) => listed.into_items(),
        Err(error) => {
            tracing::warn!(%error, "the posts could not be read for the stale-post alert");
            return Vec::new();
        }
    };
    listed
        .iter()
        .filter_map(|post| {
            let shown = serde_json::to_value(post).ok()?;
            let text = |field: &str| shown.get(field).and_then(Value::as_str).map(str::to_owned);
            let changed = chrono::DateTime::parse_from_rfc3339(&text("updated_at")?).ok()?;
            Some(LivePost {
                id: text("id")?,
                title: text("title")?,
                changed: changed.to_utc(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use chrono::{TimeZone, Utc};
    use khalid_alerts::{Alert, MemoryRaised, Raised};
    use khalid_api::not_found_log::MemoryNotFoundLog;
    use khalid_core::{Clock, Environment, FixedClock};

    use super::*;
    use crate::alerts::Fire;

    const SITE: &str = "name = \"Field Notes\"\naddress = \"https://www.example.com\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n\n[alerts]\nnot_found_spike = 3\n";

    #[tokio::test]
    async fn a_round_raises_a_burst_of_404s_since_the_last_reading() {
        let config = Config::from_toml(Path::new("site.toml"), SITE, &Environment::empty());
        let config = config.unwrap();
        let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(at));
        let fired: Arc<Mutex<Vec<Alert>>> = Arc::default();
        let kept = Arc::clone(&fired);
        let fire: Fire = Box::new(move |alert| kept.lock().unwrap().push(alert.clone()));
        let raised: Arc<dyn Raised> = Arc::new(MemoryRaised::default());
        let alerting = Alerting::new(&config, (raised, clock), None, fire);
        let log = Arc::new(MemoryNotFoundLog::default());
        let content = LiveContent::default();
        content.fail();
        let sources = Sources {
            misses: Arc::<MemoryNotFoundLog>::clone(&log),
            content,
            https: None,
            settings: config.alerts().clone(),
            watch: None,
        };
        log.record("/blog/old/", at).await.unwrap();
        let before = counts(&*sources.misses).await;
        for _ in 0..3 {
            log.record("/blog/gone/", at).await.unwrap();
        }
        let after = round(&alerting, &sources, before).await;
        let fired = fired.lock().unwrap().clone();
        assert_eq!(fired.len(), 1, "{fired:?}");
        assert_eq!(fired[0].kind(), Kind::NotFoundSpike);
        assert_eq!(fired[0].findings()[0].line, "/blog/gone/: 3 more 404s");
        let next = after.unwrap();
        assert_eq!(
            next.get("/blog/gone/"),
            Some(&3),
            "the next round starts here"
        );
    }
}
