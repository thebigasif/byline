//! One event sent to one webhook, with every attempt to send it.

use chrono::{DateTime, Utc};
use khalid_core::AbsoluteUrl;

use crate::{Event, RetrySchedule};

/// Identifies a webhook subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WebhookId(i64);

impl WebhookId {
    /// The webhook with this number, as the database stores it.
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

/// Identifies one delivery in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeliveryId(i64);

impl DeliveryId {
    /// The delivery with this number, as the store assigned it.
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// The number, for the store to write.
    pub fn get(self) -> i64 {
        self.0
    }
}

/// What came back from one attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// The receiver answered with this HTTP status.
    Answered {
        /// The status code.
        status: u16,
    },
    /// No answer, such as a refused address or a timeout.
    Unreachable {
        /// What went wrong, in words for the delivery log.
        reason: String,
    },
}

impl Outcome {
    /// Whether the receiver took the event: a 2xx answer, and nothing else.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Answered { status } if (200..300).contains(status))
    }
}

/// One try at sending a delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    at: DateTime<Utc>,
    outcome: Outcome,
}

impl Attempt {
    /// An attempt made at `at` that ended with `outcome`.
    pub fn new(at: DateTime<Utc>, outcome: Outcome) -> Self {
        Self { at, outcome }
    }
}

/// Where a delivery stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeliveryState {
    /// Waiting for its next attempt.
    Pending {
        /// When the next attempt is due.
        next_at: DateTime<Utc>,
    },
    /// The receiver took it.
    Delivered {
        /// When it succeeded.
        at: DateTime<Utc>,
    },
    /// Every retry failed, and no more will be made.
    GaveUp {
        /// When the last attempt failed.
        at: DateTime<Utc>,
    },
}

/// A delivery before the store has given it an identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDelivery {
    webhook: WebhookId,
    event: Event,
    url: AbsoluteUrl,
    body: Vec<u8>,
    first_at: DateTime<Utc>,
}

impl NewDelivery {
    /// `event` with `body` for `webhook` at `url`, first tried at `first_at`.
    pub fn new(
        webhook: WebhookId,
        event: Event,
        url: AbsoluteUrl,
        body: Vec<u8>,
        first_at: DateTime<Utc>,
    ) -> Self {
        Self {
            webhook,
            event,
            url,
            body,
            first_at,
        }
    }

    /// The stored delivery, with its identifier and no attempts yet.
    pub fn into_delivery(self, id: DeliveryId) -> Delivery {
        Delivery {
            id,
            state: DeliveryState::Pending {
                next_at: self.first_at,
            },
            webhook: self.webhook,
            event: self.event,
            url: self.url,
            body: self.body,
            attempts: Vec::new(),
        }
    }
}

/// One event for one webhook: what was sent, every attempt, and where it
/// stands. This is a row of the delivery log the admin shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    id: DeliveryId,
    webhook: WebhookId,
    event: Event,
    url: AbsoluteUrl,
    body: Vec<u8>,
    attempts: Vec<Attempt>,
    state: DeliveryState,
}

impl Delivery {
    /// The identifier in the log.
    pub fn id(&self) -> DeliveryId {
        self.id
    }

    /// The webhook it belongs to.
    pub fn webhook(&self) -> WebhookId {
        self.webhook
    }

    /// The event announced.
    pub fn event(&self) -> Event {
        self.event
    }

    /// Where it is sent.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// The JSON body, the same bytes on every attempt.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Every attempt so far, oldest first.
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }

    /// Where it stands.
    pub fn state(&self) -> &DeliveryState {
        &self.state
    }

    /// Adds `attempt` and moves the delivery on: delivered on a success,
    /// otherwise pending until the next time in `schedule`, or given up when
    /// the schedule has no more. Only a pending delivery is attempted, so
    /// every earlier attempt failed.
    pub fn record(&mut self, attempt: Attempt, schedule: &RetrySchedule) {
        let at = attempt.at;
        let success = attempt.outcome.is_success();
        self.attempts.push(attempt);
        self.state = match schedule.next(self.attempts.len(), at) {
            _ if success => DeliveryState::Delivered { at },
            Some(next_at) => DeliveryState::Pending { next_at },
            None => DeliveryState::GaveUp { at },
        };
    }

    /// Whether an attempt is due at `now`.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        matches!(self.state, DeliveryState::Pending { next_at } if next_at <= now)
    }
}
