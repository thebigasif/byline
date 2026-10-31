//! The crawl alerts in each round of `khalid serve`'s checks, while the
//! visitor log is on (GEO C4): Googlebot or Bingbot unseen for a week,
//! and server errors crawlers got in the last hour.

use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use khalid_alerts::{CrawlerSeen, Kind, ServerError, crawl_stopped, crawler_server_errors};
use khalid_analytics::{CrawledPage, VisitorLogStore, bot_errors};

use crate::alerts::Alerting;

/// The crawlers whose absence is raised: the two search engines that
/// publish their addresses. Others, such as Google's inspection tool,
/// come only when asked.
const WATCHED: [&str; 2] = ["Googlebot", "Bingbot"];

/// Runs both crawl checks at `at` against `log`. A log that cannot be read
/// is noted in the server log and raises nothing.
pub(crate) async fn round(alerting: &Alerting, log: &dyn VisitorLogStore, at: DateTime<Utc>) {
    match log.crawled().await {
        Ok(kept) => {
            let found = crawl_stopped(&last_seen(&kept), at);
            alerting.raise(Kind::CrawlStopped, found).await;
        }
        Err(error) => tracing::warn!(%error, "the crawled pages could not be read for their alert"),
    }
    let since = at - TimeDelta::hours(1);
    match log.visits(since, at).await {
        Ok(visits) => {
            let errors: Vec<ServerError> = bot_errors(&visits)
                .into_iter()
                .map(|error| ServerError {
                    path: error.path,
                    status: error.status,
                    crawlers: error
                        .crawlers
                        .iter()
                        .map(|bot| bot.name().to_owned())
                        .collect(),
                    hits: error.hits,
                })
                .collect();
            let found = crawler_server_errors(&errors, at);
            alerting.raise(Kind::CrawlerErrors, found).await;
        }
        Err(error) => tracing::warn!(%error, "the visitor log could not be read for its alert"),
    }
}

/// The latest listed fetch of each watched crawler in `kept`.
fn last_seen(kept: &[CrawledPage]) -> Vec<CrawlerSeen> {
    let mut last: BTreeMap<&str, DateTime<Utc>> = BTreeMap::new();
    for page in kept {
        let name = page.crawler.name();
        if WATCHED.contains(&name) {
            let at = last.entry(name).or_insert(page.last_at);
            *at = (*at).max(page.last_at);
        }
    }
    last.into_iter()
        .map(|(crawler, last_at)| CrawlerSeen {
            crawler: crawler.to_owned(),
            last_at,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use chrono::{TimeZone, Utc};
    use khalid_alerts::{Alert, MemoryRaised, Raised};
    use khalid_analytics::{BotCheck, Crawler, ShortAddress, Visit, Visitor, VisitorKey};
    use khalid_core::{Clock, Config, Environment, FixedClock, Slug};
    use khalid_db::{Database, VisitorLogTable};

    use super::*;
    use crate::alerts::Fire;

    const SITE: &str = "name = \"Field Notes\"\naddress = \"https://www.example.com\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";

    fn crawl(at: DateTime<Utc>, name: &str, path: &str, status: u16) -> Visit {
        let address: IpAddr = "66.249.66.1".parse().unwrap();
        let bot = Crawler::named(name).unwrap();
        Visit {
            at,
            path: path.to_owned(),
            status,
            address: ShortAddress::of(address),
            code: VisitorKey::from_bytes([4; 32]).code(address, None),
            visitor: Visitor::Bot(bot, BotCheck::Listed),
        }
    }

    #[tokio::test]
    async fn a_round_raises_server_errors_of_the_last_hour_and_bingbot_gone_quiet() {
        let dir = tempfile::tempdir().unwrap();
        let mut path = PathBuf::from(dir.path());
        path.push("site.db");
        let db = Database::open(&path).await.unwrap();
        db.migrate().await.unwrap();
        let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let log = VisitorLogTable::new(db, site);
        let at = Utc.with_ymd_and_hms(2026, 11, 9, 10, 30, 0).unwrap();
        let visits = [
            crawl(at - TimeDelta::days(8), "Bingbot", "/blog/geo/", 200),
            crawl(at - TimeDelta::hours(2), "Googlebot", "/blog/geo/", 200),
            crawl(at - TimeDelta::hours(3), "Googlebot", "/blog/old/", 500),
            crawl(at - TimeDelta::minutes(5), "GPTBot", "/blog/broken/", 503),
        ];
        log.save(&visits, DateTime::<Utc>::MIN_UTC).await.unwrap();
        let config = Config::from_toml(Path::new("site.toml"), SITE, &Environment::empty());
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(at));
        let fired: Arc<Mutex<Vec<Alert>>> = Arc::default();
        let kept = Arc::clone(&fired);
        let fire: Fire = Box::new(move |alert| kept.lock().unwrap().push(alert.clone()));
        let raised: Arc<dyn Raised> = Arc::new(MemoryRaised::default());
        let alerting = Alerting::new(&config.unwrap(), (raised, clock), None, fire);
        round(&alerting, &log, at).await;
        let fired = fired.lock().unwrap().clone();
        let lines: Vec<(Kind, &str)> = fired
            .iter()
            .flat_map(|alert| {
                let kind = alert.kind();
                alert
                    .findings()
                    .iter()
                    .map(move |f| (kind, f.line.as_str()))
            })
            .collect();
        assert_eq!(
            lines,
            [
                (
                    Kind::CrawlStopped,
                    "Bingbot last fetched a page on 2026-11-01, 8 days ago."
                ),
                (Kind::CrawlerErrors, "/blog/broken/: 503 to GPTBot, once"),
            ]
        );
    }
}
