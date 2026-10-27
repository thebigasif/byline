//! Starting HTTPS for one domain: the account, the first certificate, port
//! 80, and renewal in the background.

use std::future::Future;
use std::time::Duration;

use axum::serve::Listener;
use khalid_core::Clock;
use khalid_outbound::Outbound;
use tokio::net::TcpListener;

use crate::{
    Acme, AgreedToTerms, Certificates, Check, Directory, Domain, HttpsError, HttpsFolder, Next,
    Renewal, TlsListener, next, port_80,
};

/// The checks to answer, the TLS one first, since it needs port 443 only.
const CHECKS: [Check; 2] = [Check::TlsAlpn, Check::Http];

/// How long to wait before signing in again when Let's Encrypt cannot be
/// reached and a saved certificate is still good.
const SIGN_IN_AGAIN: Duration = Duration::from_hours(1);

/// HTTPS for one domain, as `khalid serve --domain` asks for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Https {
    /// The domain to serve and get certificates for.
    pub domain: Domain,
    /// Where the account key and the certificate are kept.
    pub folder: HttpsFolder,
    /// Where certificates come from.
    pub directory: Directory,
}

impl Https {
    /// Whether an account is saved, which means its owner agreed to the
    /// certificate authority's terms before.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`HttpsFolder::account`].
    pub async fn has_account(&self) -> Result<bool, HttpsError> {
        Ok(self.folder.account().await?.is_some())
    }

    /// Starts HTTPS. Port 80 on `plain` answers Let's Encrypt's file
    /// check and moves every visitor to HTTPS. The listener returned, on
    /// `tls`, answers with the saved certificate while it is good, or with
    /// a new one got before this returns. Renewal then runs in the
    /// background, timed by `clock`.
    ///
    /// `agreed` is the owner's yes to the terms, needed only when no
    /// account is saved.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::TermsNotAgreed`] when a new account is needed
    /// and `agreed` is `None`, and the errors of [`Acme::issue`] and
    /// [`HttpsFolder`] when the first certificate cannot be got or saved.
    pub async fn start(
        &self,
        outbound: &Outbound,
        agreed: Option<AgreedToTerms>,
        (tls, plain): (TcpListener, TcpListener),
        clock: impl Clock + 'static,
    ) -> Result<TlsListener, HttpsError> {
        let certificates = Certificates::new();
        let router = port_80(&self.domain, certificates.challenges().clone());
        tokio::spawn(async move {
            if let Err(error) = axum::serve(plain, router).await {
                tracing::warn!(%error, "port 80 stopped answering");
            }
        });
        let mut listener = TlsListener::new(tls, certificates.clone())?;
        let saved = self.folder.certificate().await?;
        let good = saved.filter(|c| next(Some(c), &self.domain, clock.now()) != Next::Renew);
        let renewal = if let Some(certificate) = good {
            certificates.set(certificate);
            None
        } else {
            let acme = self.account(outbound, agreed).await?;
            let renewal = self.renewal(acme, &certificates);
            while_accepting(&mut listener, renewal.renew()).await?;
            Some(renewal)
        };
        let https = self.clone();
        let outbound = outbound.clone();
        tokio::spawn(async move {
            let renewal = match renewal {
                Some(renewal) => renewal,
                None => {
                    https
                        .sign_in_until_it_works(&outbound, agreed, &certificates)
                        .await
                }
            };
            renewal.keep_renewed(clock).await;
        });
        Ok(listener)
    }

    /// The saved account, or a new one when `agreed` allows it.
    async fn account(
        &self,
        outbound: &Outbound,
        agreed: Option<AgreedToTerms>,
    ) -> Result<Acme, HttpsError> {
        if let Some(key) = self.folder.account().await? {
            return Acme::sign_in(outbound, key).await;
        }
        let agreed = agreed.ok_or(HttpsError::TermsNotAgreed)?;
        let (acme, key) = Acme::sign_up(outbound, &self.directory, agreed).await?;
        self.folder.save_account(&key).await?;
        Ok(acme)
    }

    /// Signs in, trying again every hour until it works.
    async fn sign_in_until_it_works(
        &self,
        outbound: &Outbound,
        agreed: Option<AgreedToTerms>,
        certificates: &Certificates,
    ) -> Renewal {
        loop {
            match self.account(outbound, agreed).await {
                Ok(acme) => return self.renewal(acme, certificates),
                Err(error) => {
                    tracing::warn!(domain = %self.domain, %error, "Let's Encrypt could not be reached, so Khalid tries again in an hour");
                    tokio::time::sleep(SIGN_IN_AGAIN).await;
                }
            }
        }
    }

    fn renewal(&self, acme: Acme, certificates: &Certificates) -> Renewal {
        Renewal {
            acme,
            domain: self.domain.clone(),
            certificates: certificates.clone(),
            folder: self.folder.clone(),
            checks: CHECKS.to_vec(),
        }
    }
}

/// Runs `work` while `listener` accepts, so Let's Encrypt's TLS check is
/// answered before the site is served. A visitor in that time is turned
/// away, since there is no certificate to answer with yet.
async fn while_accepting<T>(listener: &mut TlsListener, work: impl Future<Output = T>) -> T {
    tokio::pin!(work);
    loop {
        tokio::select! {
            done = &mut work => return done,
            accepted = listener.accept() => drop(accepted),
        }
    }
}
