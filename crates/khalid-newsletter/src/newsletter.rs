//! The sign-up flow: a form makes a subscriber wait, one email asks them,
//! and a link confirms or removes them.

use std::sync::Arc;

use khalid_core::{Clock, Locale, Slug};

use crate::{
    BackPath, ConfirmationLinks, Confirmed, EmailAddress, GenerateTokenError, MailError, Mailer,
    NewsletterWords, ReaderPass, SignUp, SignedUp, SiteLinks, StoreError, SubscriberStore,
    SubscriberToken, Unsubscribed, WordsError,
};

/// The site the emails come from: its name, its language and where its
/// pages answer.
#[derive(Debug, Clone)]
pub struct NewsletterSite {
    /// The site's name, as the emails name it.
    pub name: String,
    /// The language the emails are written in.
    pub locale: Locale,
    /// Where the links in the emails lead.
    pub links: SiteLinks,
}

/// One site's newsletter: its subscribers, how mail leaves, and its clock
/// (RULES.md BL-RUN-1). Cloning shares all three.
#[derive(Debug, Clone)]
pub struct Newsletter {
    store: Arc<dyn SubscriberStore>,
    mailer: Arc<dyn Mailer>,
    clock: Arc<dyn Clock>,
    site: Arc<(NewsletterSite, NewsletterWords)>,
    pass: Option<ReaderPass>,
}

impl Newsletter {
    /// The newsletter of `site`, keeping subscribers in `store` and
    /// sending through `mailer`.
    ///
    /// # Errors
    ///
    /// [`WordsError`] when a built-in words file is not valid, which this
    /// crate's tests rule out.
    pub fn new(
        store: Arc<dyn SubscriberStore>,
        mailer: Arc<dyn Mailer>,
        clock: Arc<dyn Clock>,
        site: NewsletterSite,
    ) -> Result<Self, WordsError> {
        let words = NewsletterWords::load()?;
        Ok(Self {
            store,
            mailer,
            clock,
            site: Arc::new((site, words)),
            pass: None,
        })
    }

    /// This newsletter, giving each reader who confirms the cookie `pass`
    /// makes, so they can read gated content.
    #[must_use]
    pub fn with_pass(mut self, pass: ReaderPass) -> Self {
        self.pass = Some(pass);
        self
    }

    /// The reader pass, when gated content opens for subscribers.
    pub fn pass(&self) -> Option<&ReaderPass> {
        self.pass.as_ref()
    }

    /// Signs `email` up for `list` from the gated content at `back`, as
    /// [`sign_up`](Self::sign_up) does, with `back` in the confirmation
    /// link.
    ///
    /// # Errors
    ///
    /// [`NewsletterError`] as [`sign_up`](Self::sign_up) returns it.
    pub async fn sign_up_to_read(
        &self,
        email: EmailAddress,
        list: Slug,
        back: &BackPath,
    ) -> Result<SignedUp, NewsletterError> {
        self.signed_up(email, list, Some(back)).await
    }

    /// Signs `email` up for `list`. When a confirmation is due, sends it,
    /// with a link that confirms and a link that removes the address.
    /// The reader is told the same thing whatever the answer, so the form
    /// never reveals who subscribes.
    ///
    /// # Errors
    ///
    /// [`NewsletterError`] when there is no randomness for a token, the
    /// store fails, or the transport does not take the email. A failed
    /// email can be asked for again after ten minutes.
    pub async fn sign_up(
        &self,
        email: EmailAddress,
        list: Slug,
    ) -> Result<SignedUp, NewsletterError> {
        self.signed_up(email, list, None).await
    }

    /// Signs up as [`sign_up`](Self::sign_up) says, with `back` in the link.
    async fn signed_up(
        &self,
        email: EmailAddress,
        list: Slug,
        back: Option<&BackPath>,
    ) -> Result<SignedUp, NewsletterError> {
        let (confirm, remove) = (SubscriberToken::generate()?, SubscriberToken::generate()?);
        let at = self.clock.now();
        let asked = SignUp::new(email.clone(), list, confirm.hash(), at);
        let signed = self.store.sign_up(&asked).await?;
        if let SignedUp::ConfirmationDue(subscriber) = signed {
            self.store
                .add_unsubscribe(subscriber, &remove.hash(), at)
                .await?;
            let (site, words) = &*self.site;
            let links = ConfirmationLinks {
                site: &site.links,
                confirm: &confirm,
                remove: &remove,
                back,
            };
            let mail = words.confirmation(&site.locale, &site.name, email, links);
            self.mailer.send(&mail).await?;
        }
        Ok(signed)
    }

    /// Confirms the subscriber whose link holds `token`, as typed in the
    /// link. Text that is not a token confirms nobody.
    ///
    /// # Errors
    ///
    /// [`NewsletterError::Store`] when the store fails.
    pub async fn confirm(&self, token: &str) -> Result<Confirmed, NewsletterError> {
        let Ok(token) = SubscriberToken::parse(token) else {
            return Ok(Confirmed::Unknown);
        };
        Ok(self.store.confirm(&token.hash(), self.clock.now()).await?)
    }

    /// Unsubscribes the subscriber whose link holds `token`, as typed in
    /// the link. Text that is not a token unsubscribes nobody.
    ///
    /// # Errors
    ///
    /// [`NewsletterError::Store`] when the store fails.
    pub async fn unsubscribe(&self, token: &str) -> Result<Unsubscribed, NewsletterError> {
        let Ok(token) = SubscriberToken::parse(token) else {
            return Ok(Unsubscribed::Unknown);
        };
        Ok(self
            .store
            .unsubscribe(&token.hash(), self.clock.now())
            .await?)
    }
}

/// Why a sign-up, a confirmation or an unsubscribe did not happen.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NewsletterError {
    /// The operating system gave no randomness for a link.
    #[error(transparent)]
    Random(#[from] GenerateTokenError),
    /// The subscriber list could not be read or written.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The transport did not take the confirmation email.
    #[error(transparent)]
    Mail(#[from] MailError),
}
