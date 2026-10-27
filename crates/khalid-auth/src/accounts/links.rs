//! Signing in by a link sent by email.

use std::net::IpAddr;
use std::sync::Arc;

use super::{Accounts, SignInError, SignedIn};
use crate::{Email, LinkSender, LinkStore, LinkToken, SessionToken};

/// What link sign-in needs: where links are kept, what mails them, and the
/// admin's address the links point into, such as
/// `https://blog.example.com/admin`.
#[derive(Clone)]
pub struct Links {
    /// Where links are kept.
    pub store: Arc<dyn LinkStore>,
    /// What mails them, or `None` while the site has no mail sender. Links
    /// made another way, such as by `khalid staff owner`, still work.
    pub sender: Option<Arc<dyn LinkSender>>,
    /// The admin's address.
    pub admin: String,
}

impl std::fmt::Debug for Links {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Links")
            .field("admin", &self.admin)
            .finish_non_exhaustive()
    }
}

impl Accounts {
    /// These accounts, able to sign in by links as `links` says.
    #[must_use]
    pub fn with_links(mut self, links: Links) -> Self {
        self.links = Some(links);
        self
    }

    /// Mails a sign-in link to `email` when it has an account. The answer
    /// is the same either way, so nobody learns which emails have
    /// accounts; each request counts toward the sign-in limit, so nobody
    /// floods an inbox.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::LinksOff`] when the site cannot send mail,
    /// [`SignInError::Limited`] after too many requests, and
    /// [`SignInError::Unavailable`] when a store or the sender fails.
    pub async fn send_link(&self, email: &Email, from: Option<IpAddr>) -> Result<(), SignInError> {
        let links = self.links.as_ref().ok_or(SignInError::LinksOff)?;
        let sender = links.sender.as_ref().ok_or(SignInError::LinksOff)?;
        let now = self.clock.now();
        if let Some(until) = self.limiter.blocked_until(email, from, now) {
            return Err(SignInError::limited(until - now));
        }
        self.limiter.failed(email, from, now);
        let Some(stored) = self.store.find_by_email(email).await? else {
            return Ok(());
        };
        let token = LinkToken::generate()?;
        links
            .store
            .create(token.hash(), stored.account.id(), now)
            .await?;
        sender.send(email, token.link(&links.admin)).await?;
        Ok(())
    }

    /// Signs in with the token from a sign-in link, once: the link stops
    /// working when used or when [`LINK_LIFETIME`](crate::LINK_LIFETIME)
    /// passes.
    ///
    /// # Errors
    ///
    /// Returns [`SignInError::Refused`] for a used, expired or unknown
    /// link, [`SignInError::LinksOff`] when the site has no link sign-in,
    /// and [`SignInError::Unavailable`] when a store fails.
    pub async fn redeem_link(
        &self,
        token: &LinkToken,
    ) -> Result<(SessionToken, SignedIn), SignInError> {
        let links = self.links.as_ref().ok_or(SignInError::LinksOff)?;
        let now = self.clock.now();
        let Some(id) = links.store.redeem(token.hash(), now).await? else {
            return Err(SignInError::Refused);
        };
        let account = self.store.find(id).await?.ok_or(SignInError::Refused)?;
        self.limiter.succeeded(account.email());
        let session = self.begin(&account).await?;
        let signed_in = SignedIn {
            account,
            csrf: session.csrf(),
            signed_in_at: now,
        };
        Ok((session, signed_in))
    }
}
