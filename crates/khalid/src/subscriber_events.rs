//! The `subscriber.added` webhook (audit brief G25): fired when a reader
//! opens their confirmation link and becomes a subscriber, not when they
//! first fill in the form, since an address nobody confirmed gets no mail.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_core::Slug;
use khalid_newsletter::{
    BoxFuture, Confirmed, EmailAddress, Labelled, Member, Recipient, Segment, SendId, SendNumbers,
    SendTotals, SignUp, SignedUp, StoreError, SubscriberId, SubscriberStore, TokenHash,
    Unsubscribed,
};
use khalid_webhooks::Event;
use serde::Serialize;

use crate::announce::SiteAnnounce;

/// A subscriber store that tells `announce` of each subscriber confirmed,
/// and otherwise does what the store it wraps does.
#[derive(Debug)]
pub(crate) struct AnnouncedSubscribers {
    store: Arc<dyn SubscriberStore>,
    announce: SiteAnnounce,
}

/// The resource a `subscriber.added` delivery carries. The address stays
/// out, as a reader's email does in `comment.added`: the webhook says who
/// joined by number and when.
#[derive(Debug, Serialize)]
struct Added {
    subscriber: Subscriber,
}

#[derive(Debug, Serialize)]
struct Subscriber {
    id: i64,
    confirmed_at: DateTime<Utc>,
}

impl AnnouncedSubscribers {
    /// Wraps `store`, telling `announce`.
    pub(crate) fn new(store: Arc<dyn SubscriberStore>, announce: SiteAnnounce) -> Self {
        Self { store, announce }
    }
}

impl SubscriberStore for AnnouncedSubscribers {
    fn sign_up<'a>(&'a self, sign_up: &'a SignUp) -> BoxFuture<'a, Result<SignedUp, StoreError>> {
        self.store.sign_up(sign_up)
    }

    fn confirm<'a>(
        &'a self,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Confirmed, StoreError>> {
        Box::pin(async move {
            let confirmed = self.store.confirm(token, at).await?;
            if let Confirmed::Now(id) = confirmed {
                let subscriber = Subscriber {
                    id: id.as_row(),
                    confirmed_at: at,
                };
                self.announce
                    .fire(Event::SubscriberAdded, Added { subscriber });
            }
            Ok(confirmed)
        })
    }

    fn add_unsubscribe<'a>(
        &'a self,
        subscriber: SubscriberId,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        self.store.add_unsubscribe(subscriber, token, at)
    }

    fn unsubscribe<'a>(
        &'a self,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Unsubscribed, StoreError>> {
        self.store.unsubscribe(token, at)
    }

    fn label<'a>(
        &'a self,
        email: &'a EmailAddress,
        label: &'a Slug,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Labelled, StoreError>> {
        self.store.label(email, label, at)
    }

    fn unlabel<'a>(
        &'a self,
        email: &'a EmailAddress,
        label: &'a Slug,
    ) -> BoxFuture<'a, Result<Labelled, StoreError>> {
        self.store.unlabel(email, label)
    }

    fn start_send<'a>(
        &'a self,
        key: &'a str,
        list: &'a Slug,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<SendId>, StoreError>> {
        self.store.start_send(key, list, at)
    }

    fn add_send_unsubscribe<'a>(
        &'a self,
        send: SendId,
        subscriber: SubscriberId,
        token: &'a TokenHash,
        at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        self.store.add_send_unsubscribe(send, subscriber, token, at)
    }

    fn finish_send(
        &self,
        send: SendId,
        totals: SendTotals,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), StoreError>> {
        self.store.finish_send(send, totals, at)
    }

    fn sends(&self) -> BoxFuture<'_, Result<Vec<SendNumbers>, StoreError>> {
        self.store.sends()
    }

    fn subscriber(
        &self,
        subscriber: SubscriberId,
    ) -> BoxFuture<'_, Result<Option<Member>, StoreError>> {
        self.store.subscriber(subscriber)
    }

    fn audience<'a>(
        &'a self,
        segment: &'a Segment,
    ) -> BoxFuture<'a, Result<Vec<Recipient>, StoreError>> {
        self.store.audience(segment)
    }
}
