//! The sign-up flow: a form makes a subscriber wait, one email asks them,
//! and a link confirms or removes them. Then the issues that go out to them.

use std::sync::Arc;

use khalid_core::{Clock, Locale, Slug};

use crate::{
    BackPath, ConfirmationLinks, Confirmed, EmailAddress, GenerateTokenError, Issue, Lists, Mail,
    MailError, Mailer, Member, NewsletterWords, PostalAddress, ReaderPass, Segment, SendTotals,
    Sent, SignUp, SignedUp, SiteLinks, StoreError, SubscriberId, SubscriberStore, SubscriberToken,
    Unsubscribed, Welcomes, WordsError,
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
    lists: Arc<Lists>,
    welcomes: Option<Arc<Welcomes>>,
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
            lists: Arc::new(Lists::main_only()),
            welcomes: None,
        })
    }

    /// The same newsletter, taking sign-ups for `lists` instead of `main`
    /// alone.
    #[must_use]
    pub fn with_lists(self, lists: Lists) -> Self {
        Self {
            lists: Arc::new(lists),
            ..self
        }
    }

    /// The site's newsletters.
    pub fn lists(&self) -> &Lists {
        &self.lists
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
    /// link. An address confirmed before gets a pass link to `back`
    /// instead, so a subscriber on a new device can read too.
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
        let signed = self.signed_up(email.clone(), list, Some(back)).await?;
        if let (SignedUp::AlreadyConfirmed, Some(pass)) = (&signed, &self.pass) {
            let (site, words) = &*self.site;
            let mail = words.pass(&site.locale, &site.name, email, &pass.link(back));
            self.mailer.send(&mail).await?;
        }
        Ok(signed)
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

    /// The same newsletter, sending `welcomes` to each reader the moment
    /// they confirm.
    #[must_use]
    pub fn with_welcomes(self, welcomes: Welcomes) -> Self {
        Self {
            welcomes: Some(Arc::new(welcomes)),
            ..self
        }
    }

    /// Sends `subscriber`, confirmed a moment ago, the welcome email of each
    /// newsletter they joined that has one. A welcome that cannot go out is
    /// logged and left: the reader is subscribed all the same.
    async fn welcome(&self, subscriber: SubscriberId) {
        let Some(welcomes) = &self.welcomes else {
            return;
        };
        let found = match self.store.subscriber(subscriber).await {
            Ok(Some(found)) => found,
            Ok(None) => return,
            Err(error) => return tracing::warn!(%error, "a welcome email was not sent"),
        };
        let Member {
            recipient, lists, ..
        } = found;
        let (site, words) = &*self.site;
        for welcome in lists.iter().filter_map(|list| welcomes.of(list)) {
            let sent = async {
                let remove = SubscriberToken::generate()?;
                let at = self.clock.now();
                self.store
                    .add_unsubscribe(recipient.id, &remove.hash(), at)
                    .await?;
                let link = site.links.unsubscribe(&remove);
                let footer = words.footer(&site.locale, &site.name, &link, welcomes.postal());
                let text = format!("{}{footer}", welcome.text());
                let to = recipient.email.clone();
                let mail = Mail::new(to, welcome.subject().to_owned(), text);
                Ok::<_, NewsletterError>(self.mailer.send(&mail).await?)
            };
            if let Err(error) = sent.await {
                tracing::warn!(%error, "a welcome email was not sent");
            }
        }
    }

    /// Confirms the subscriber whose link holds `token`, as typed in the
    /// link, and sends the welcome emails of the newsletters they joined.
    /// Text that is not a token confirms nobody.
    ///
    /// # Errors
    ///
    /// [`NewsletterError::Store`] when the store fails.
    pub async fn confirm(&self, token: &str) -> Result<Confirmed, NewsletterError> {
        let Ok(token) = SubscriberToken::parse(token) else {
            return Ok(Confirmed::Unknown);
        };
        let confirmed = self.store.confirm(&token.hash(), self.clock.now()).await?;
        if let Confirmed::Now(subscriber) = confirmed {
            self.welcome(subscriber).await;
        }
        Ok(confirmed)
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

impl Newsletter {
    /// Emails `issue` to everyone in `segment`, once: an issue whose key
    /// went out before sends nothing. Each copy ends with the reader's own
    /// one-click unsubscribe link and the site's `postal` address. A copy
    /// the transport refuses is counted and the rest still go.
    ///
    /// # Errors
    ///
    /// [`NewsletterError`] when there is no randomness for a link or the
    /// store fails. The copies sent before a failure stay sent, and the
    /// key stays used, so a second try never sends anyone a copy twice.
    pub async fn send(
        &self,
        issue: &Issue,
        segment: &Segment,
        postal: &PostalAddress,
    ) -> Result<Sent, NewsletterError> {
        let at = self.clock.now();
        let Some(send) = self
            .store
            .start_send(issue.key(), segment.list(), at)
            .await?
        else {
            return Ok(Sent::Before);
        };
        let (site, words) = &*self.site;
        let mut totals = SendTotals::default();
        for recipient in self.store.audience(segment).await? {
            let remove = SubscriberToken::generate()?;
            self.store
                .add_send_unsubscribe(send, recipient.id, &remove.hash(), at)
                .await?;
            let link = site.links.unsubscribe(&remove);
            let footer = words.footer(&site.locale, &site.name, &link, postal);
            let text = format!("{}{footer}", issue.text());
            let mail = Mail::new(recipient.email, issue.subject().to_owned(), text);
            match self.mailer.send(&mail).await {
                Ok(()) => totals.sent = totals.sent.saturating_add(1),
                Err(error) => {
                    tracing::warn!(%error, "a newsletter copy was not sent");
                    totals.failed = totals.failed.saturating_add(1);
                }
            }
        }
        self.store
            .finish_send(send, totals, self.clock.now())
            .await?;
        Ok(Sent::Now(totals))
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
