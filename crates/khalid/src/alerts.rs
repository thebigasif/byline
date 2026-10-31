//! Alerts for the site's owner (KH-GRO-6): each new finding goes out once,
//! by email to `[alerts] email` through `[mail]` when the site has both,
//! and always to the webhooks that asked for `alert.raised`. The server
//! log says each one too, so an owner with neither still sees it.

use std::fmt;
use std::sync::Arc;

use khalid_alerts::{Alert, Finding, Kind, Raised, fresh};
use khalid_core::{Clock, Config};
use khalid_newsletter::{EmailAddress, Mail, Mailer};

/// Where an alert's webhook delivery goes: the site's announcer in
/// `khalid serve`, a list in tests.
pub(crate) type Fire = Box<dyn Fn(&Alert) + Send + Sync>;

/// Raises alerts for one site. Cloning is not needed: it is shared in an
/// `Arc`.
pub(crate) struct Alerting {
    site: String,
    raised: Arc<dyn Raised>,
    clock: Arc<dyn Clock>,
    email: Option<(Arc<dyn Mailer>, EmailAddress)>,
    fire: Fire,
}

impl fmt::Debug for Alerting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Alerting").finish_non_exhaustive()
    }
}

impl Alerting {
    /// Alerts for the site with `config`, keeping what was raised in
    /// `raised` at the times `clock` gives, emailing through `mailer` when
    /// `[alerts] email` names an owner, and firing each through `fire`.
    pub(crate) fn new(
        config: &Config,
        (raised, clock): (Arc<dyn Raised>, Arc<dyn Clock>),
        mailer: Option<Arc<dyn Mailer>>,
        fire: Fire,
    ) -> Self {
        let owner = config.alerts().email.as_deref().map(EmailAddress::parse);
        // The address was checked at start, so it always parses.
        let email = match (mailer, owner) {
            (Some(mailer), Some(Ok(owner))) => Some((mailer, owner)),
            _ => None,
        };
        Self {
            site: config.name().to_owned(),
            raised,
            clock,
            email,
            fire,
        }
    }

    /// Sends the `findings` of `kind` not raised before, as one alert. A
    /// failure is logged; the webhook log retries its own deliveries.
    pub(crate) async fn raise(&self, kind: Kind, findings: Vec<Finding>) {
        let fresh = fresh(&*self.raised, kind, findings, self.clock.now()).await;
        match fresh {
            Ok(Some(alert)) => self.send(&alert).await,
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, kind = kind.as_str(), "an alert check could not be saved, so it runs again next round");
            }
        }
    }

    /// Drops the raised keys older than 30 days, so the table stays small.
    pub(crate) async fn tidy(&self) {
        let before = self.clock.now() - chrono::Duration::days(30);
        if let Err(error) = self.raised.forget_before(before).await {
            tracing::warn!(%error, "old alerts could not be dropped");
        }
    }

    async fn send(&self, alert: &Alert) {
        let kind = alert.kind().as_str();
        let count = alert.findings().len();
        tracing::warn!(kind, count, "an alert was raised for the site's owner");
        (self.fire)(alert);
        let Some((mailer, owner)) = &self.email else {
            return;
        };
        let mail = Mail::new(owner.clone(), alert.subject(&self.site), alert.text());
        if let Err(error) = mailer.send(&mail).await {
            tracing::warn!(%error, kind, "an alert could not be emailed");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use chrono::{TimeZone, Utc};
    use khalid_alerts::MemoryRaised;
    use khalid_core::{Environment, FixedClock};
    use khalid_newsletter::MemoryMailer;

    use super::*;

    const SITE: &str = "name = \"Field Notes\"\naddress = \"https://www.example.com\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n\n[alerts]\nemail = \"owner@example.com\"\n\n[mail]\nfrom = \"news@example.com\"\n\n[mail.resend]\n";

    fn stale(id: &str) -> Vec<Finding> {
        let key = Kind::StalePost.key(id);
        vec![Finding {
            key,
            line: format!("Post {id}"),
        }]
    }

    #[tokio::test]
    async fn a_new_finding_is_emailed_and_fired_once() {
        let env = Environment::from_pairs(&[(Environment::RESEND_API_KEY, "made-for-this-test")]);
        let config = Config::from_toml(Path::new("site.toml"), SITE, &env).unwrap();
        let mailer = MemoryMailer::default();
        let fired = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&fired);
        let fire: Fire = Box::new(move |alert| kept.lock().unwrap().push(alert.kind()));
        let at = Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::new(at));
        let raised: Arc<dyn Raised> = Arc::new(MemoryRaised::default());
        let store = (raised, clock);
        let alerting = Alerting::new(&config, store, Some(Arc::new(mailer.clone())), fire);
        alerting.raise(Kind::StalePost, stale("en.a")).await;
        alerting.raise(Kind::StalePost, stale("en.a")).await;
        let sent = mailer.sent();
        assert_eq!(sent.len(), 1, "once");
        assert_eq!(sent[0].to.as_str(), "owner@example.com");
        assert_eq!(
            sent[0].subject,
            "Field Notes: Some live posts have not changed for a long time"
        );
        assert!(sent[0].text.ends_with("- Post en.a\n"), "{}", sent[0].text);
        assert_eq!(*fired.lock().unwrap(), [Kind::StalePost]);
    }
}
