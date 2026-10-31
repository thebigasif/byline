//! The speed and uptime watch (C6): `khalid serve` looks at the blog's
//! home page at its public address every few minutes, as a reader would,
//! through DNS, the certificate and any proxy in front. Each look is kept
//! for 30 days, and three failed looks in a row, or a slow hour, raise an
//! alert. A server that is down cannot say so itself: its looks stop.

use std::time::Duration;

use chrono::{DateTime, Duration as Days, Utc};
use khalid_alerts::{Answer, Kind, Probe, site_down, slow_site};
use khalid_core::{AbsoluteUrl, Config};
use khalid_db::{Database, DbError, SiteId, WatchTable};
use khalid_outbound::{Outbound, OutboundError};
use tokio::time::Instant;

use crate::alerts::Alerting;
use crate::cli::ServeArgs;

/// Looks are kept this many days.
pub(crate) const KEPT_DAYS: i64 = 30;

/// The looks read back: three at the rarest watch, and a margin.
const READ_BACK: Days = Days::hours(4);

/// How often the watch looks for `args` and the site with `config`, or
/// `None`: every `[alerts] watch_minutes`, but never while the server
/// answers only this computer, as it does by default while writing. A
/// test build may ask with the hidden `--watch-every`.
pub(crate) fn every(args: &ServeArgs, config: &Config) -> Option<Duration> {
    #[cfg(feature = "test-util")]
    if let Some(seconds) = args.watch_every {
        return Some(Duration::from_secs(seconds));
    }
    let minutes = config.alerts().watch_minutes;
    let only_here = args.domain.is_none() && args.listen.ip().is_loopback();
    if minutes == 0 || only_here {
        return None;
    }
    Some(Duration::from_secs(u64::from(minutes) * 60))
}

/// The blog's home page at its public address, and where its looks go.
#[derive(Debug)]
pub(crate) struct SiteWatch {
    outbound: Outbound,
    page: AbsoluteUrl,
    table: WatchTable,
    slow_ms: u32,
}

impl SiteWatch {
    /// The watch of the site with `config`, kept in site `id` of
    /// `database`, looking through `outbound`. `None` when the address and
    /// the mount make no address, which the settings check refuses first.
    pub(crate) fn new(
        config: &Config,
        outbound: &Outbound,
        (database, id): (&Database, SiteId),
    ) -> Option<Self> {
        let base = config.address().as_str().trim_end_matches('/');
        let page = AbsoluteUrl::parse(&format!("{base}{}", config.mount())).ok()?;
        Some(Self {
            outbound: outbound.clone(),
            page,
            table: WatchTable::new(database.clone(), id),
            slow_ms: config.alerts().slow_ms,
        })
    }

    /// Looks once, keeps the look, and raises what the recent looks show
    /// through `alerting`. A failure to keep or read looks is logged, and
    /// the next round tries again.
    pub(crate) async fn round(&self, alerting: &Alerting) {
        let at = alerting.now();
        let Some(probe) = self.look(at).await else {
            return;
        };
        let page = self.page.as_str();
        let Ok(probes) = self.keep(&probe, at).await.inspect_err(|error| {
            tracing::warn!(%error, "the speed and uptime watch could not keep its looks");
        }) else {
            return;
        };
        alerting
            .raise(Kind::SiteDown, site_down(&probes, page))
            .await;
        let slow = slow_site(&probes, self.slow_ms, page, at);
        alerting.raise(Kind::SlowSite, slow).await;
    }

    /// Keeps `probe`, drops looks older than 30 days at `at`, and reads
    /// back the recent ones, oldest first.
    async fn keep(&self, probe: &Probe, at: DateTime<Utc>) -> Result<Vec<Probe>, DbError> {
        let page = self.page.as_str();
        self.table.record(page, probe).await?;
        self.table.forget_before(at - Days::days(KEPT_DAYS)).await?;
        self.table.since(page, at - READ_BACK).await
    }

    /// One look at the page, started `at`: its status and how long the
    /// whole answer took. `None` when the guard refuses the address,
    /// because it is on a private network: no reader outside sees it, so
    /// there is nothing to watch, and the log says so.
    async fn look(&self, at: DateTime<Utc>) -> Option<Probe> {
        let started = Instant::now();
        let answer = match self.outbound.get(&self.page).await {
            Ok(fetched) => Answer::Status(fetched.status()),
            Err(OutboundError::Blocked { .. }) => {
                tracing::warn!(
                    page = self.page.as_str(),
                    "the speed and uptime watch does not look at a private address"
                );
                return None;
            }
            Err(_) => Answer::Unreached,
        };
        let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
        Some(Probe { at, answer, millis })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::Parser;
    use khalid_core::Environment;

    use super::*;
    use crate::cli::{Cli, Command};

    const SITE: &str = "name = \"Field Notes\"\naddress = \"https://www.example.com\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";

    fn every_for(line: &[&str], alerts: &str) -> Option<Duration> {
        let text = format!("{SITE}{alerts}");
        let config = Config::from_toml(Path::new("site.toml"), &text, &Environment::empty());
        let cli = Cli::try_parse_from(line).unwrap();
        let Some(Command::Serve(args)) = cli.command else {
            panic!("not serve");
        };
        every(&args, &config.unwrap())
    }

    #[test]
    fn the_watch_looks_only_when_the_server_answers_more_than_this_computer() {
        let public = ["khalid", "serve", "--listen", "0.0.0.0:8080"];
        assert_eq!(every_for(&public, ""), Some(Duration::from_secs(300)));
        let rare = "\n[alerts]\nwatch_minutes = 60\n";
        assert_eq!(every_for(&public, rare), Some(Duration::from_secs(3600)));
        assert_eq!(every_for(&public, "\n[alerts]\nwatch_minutes = 0\n"), None);
        let https = ["khalid", "serve", "--domain", "blog.example.com"];
        assert_eq!(every_for(&https, ""), Some(Duration::from_secs(300)));
        assert_eq!(every_for(&["khalid", "serve"], ""), None, "writing here");
        let here = ["khalid", "serve", "--listen", "[::1]:8080"];
        assert_eq!(every_for(&here, ""), None);
    }
}
