//! The `subscriber.added` webhook (audit brief G25): fired when a reader
//! opens their confirmation link and becomes a subscriber, not when they
//! first fill in the form, since an address nobody confirmed gets no mail.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_newsletter::{
    BoxFuture, Confirmed, SignUp, SignedUp, StoreError, SubscriberId, SubscriberStore, TokenHash,
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
}
