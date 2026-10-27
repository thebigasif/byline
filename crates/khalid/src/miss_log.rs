//! The 404s `khalid serve` answers, handed to one task that writes them to
//! the log in `data/site.db`, so a request never waits for the database
//! (KH-SEO-8).

use std::sync::Arc;

use khalid_api::not_found_log::NotFoundLog;
use khalid_core::Clock;
use tokio::sync::mpsc;

/// The most 404s waiting to be written. More are dropped, so a burst of
/// requests for missing addresses cannot pile up in memory.
const WAITING: usize = 256;

/// The longest address logged, in bytes. Longer ones are bot noise, and
/// left out.
const LONGEST: usize = 512;

/// Where the layer hands each 404. Cloning shares the one writer.
#[derive(Debug, Clone)]
pub(crate) struct MissLog(mpsc::Sender<String>);

impl MissLog {
    /// Starts the writer, which records each address in `log` at the time
    /// `clock` gives, and stops once every `MissLog` is dropped.
    pub(crate) fn start(log: Arc<dyn NotFoundLog>, clock: Arc<dyn Clock>) -> Self {
        let (sender, mut addresses) = mpsc::channel::<String>(WAITING);
        tokio::spawn(async move {
            while let Some(path) = addresses.recv().await {
                if let Err(error) = log.record(&path, clock.now()).await {
                    tracing::warn!(%error, "a 404 was not logged");
                }
            }
        });
        Self(sender)
    }

    /// Hands `path` to the writer, or drops it when the writer is busy or
    /// the address is too long.
    pub(crate) fn note(&self, path: &str) {
        if path.len() <= LONGEST && self.0.try_send(path.to_owned()).is_err() {
            tracing::debug!("a 404 was dropped while the log was busy");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use khalid_api::not_found_log::MemoryNotFoundLog;
    use khalid_core::FixedClock;

    use super::*;

    #[tokio::test]
    async fn each_noted_address_reaches_the_log_and_long_ones_do_not() {
        let log = Arc::new(MemoryNotFoundLog::default());
        let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
        let shared: Arc<dyn NotFoundLog> = Arc::<MemoryNotFoundLog>::clone(&log);
        let misses = MissLog::start(shared, Arc::new(FixedClock::new(at)));
        misses.note("/blog/internal-links/");
        misses.note(&format!("/blog/{}/", "a".repeat(LONGEST)));
        misses.note("/blog/internal-links/");
        for _ in 0..100 {
            let logged = log.misses(10).await.unwrap();
            if logged.first().is_some_and(|miss| miss.hits == 2) {
                assert_eq!(logged.len(), 1);
                assert_eq!(logged[0].last_seen, at);
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the 404s never reached the log");
    }
}
