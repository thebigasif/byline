//! The sign-up flow: a form makes a subscriber wait, one email asks them,
//! and a link confirms or removes them. Then the issues that go out to them.

use std::sync::Arc;

use khalid_core::{Clock, Locale, Slug};

use crate::{
    ConfirmationLinks, Confirmed, EmailAddress, GenerateTokenError, Issue, Lists, Mail, MailError,
    Mailer, NewsletterWords, PostalAddress, Segment, SendTotals, Sent, SignUp, SignedUp, SiteLinks,
    StoreError, SubscriberStore, SubscriberToken, Unsubscribed, WordsError,
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
    lists: Arc<Lists>,
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
            lists: Arc::new(Lists::main_only()),
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
