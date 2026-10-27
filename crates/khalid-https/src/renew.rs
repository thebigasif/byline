//! Keeping the certificate renewed while the server runs.

use std::time::Duration;

use chrono::{DateTime, Utc};
use khalid_core::Clock;

use crate::{Acme, Certificate, Certificates, Check, Domain, HttpsError, HttpsFolder};

/// The longest wait between two looks at the certificate, so a changed
/// clock or a lost file is noticed within half a day.
const LOOK_EVERY: Duration = Duration::from_hours(12);

/// How long to wait after a failed renewal before trying again. Let's
/// Encrypt limits failed orders, and a 30-day margin leaves many tries.
const RETRY_AFTER: Duration = Duration::from_hours(1);

/// What to do next about the certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Get a new certificate now.
    Renew,
    /// Look again after this long.
    Wait(Duration),
}

/// What to do about `current` for `domain` at `now`: renew when there is
/// none, when it is for another name, or once its renewal time has come.
pub fn next(current: Option<&Certificate>, domain: &Domain, now: DateTime<Utc>) -> Next {
    match current {
        Some(certificate)
            if certificate.covers(domain.as_str()) && now < certificate.renew_at() =>
        {
            let left = (certificate.renew_at() - now)
                .to_std()
                .unwrap_or(Duration::ZERO);
            Next::Wait(left.min(LOOK_EVERY))
        }
        _ => Next::Renew,
    }
}

/// Everything a renewal needs: the account, the domain, where the
/// certificate is served from and saved to, and the checks to answer.
#[derive(Debug)]
pub struct Renewal {
    /// The account to order with.
    pub acme: Acme,
    /// The domain the certificate is for.
    pub domain: Domain,
    /// Where the listener reads the certificate from.
    pub certificates: Certificates,
    /// Where the certificate is saved.
    pub folder: HttpsFolder,
    /// The checks to answer, the first the authority offers.
    pub checks: Vec<Check>,
}

impl Renewal {
    /// Gets a new certificate, saves it, and serves it from the next
    /// connection.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Acme::issue`] and
    /// [`HttpsFolder::save_certificate`]; the certificate being served
    /// stays as it was.
    pub async fn renew(&self) -> Result<Certificate, HttpsError> {
        let challenges = self.certificates.challenges();
        let issued = self
            .acme
            .issue(&self.domain, challenges, &self.checks)
            .await?;
        let certificate = self.folder.save_certificate(&issued).await?;
        self.certificates.set(certificate.clone());
        Ok(certificate)
    }

    /// Renews whenever [`next`] says so, reading the time from `clock`,
    /// until the task running it is dropped. A failed renewal is logged
    /// and tried again an hour later.
    pub async fn keep_renewed(self, clock: impl Clock) {
        loop {
            let current = self.certificates.current();
            match next(current.as_ref(), &self.domain, clock.now()) {
                Next::Wait(wait) => tokio::time::sleep(wait).await,
                Next::Renew => match self.renew().await {
                    Ok(certificate) => {
                        tracing::info!(domain = %self.domain, until = %certificate.not_after(), "the HTTPS certificate was renewed");
                    }
                    Err(error) => {
                        tracing::warn!(domain = %self.domain, %error, "the HTTPS certificate could not be renewed, so Khalid tries again in an hour");
                        tokio::time::sleep(RETRY_AFTER).await;
                    }
                },
            }
        }
    }
}
