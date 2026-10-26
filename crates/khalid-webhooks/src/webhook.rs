//! A subscription: where to send which events, and the key to sign them.

use khalid_core::AbsoluteUrl;

use crate::{Event, SigningKey, WebhookId};

/// A receiver that asked for some events. The Admin API manages these.
#[derive(Debug, Clone)]
pub struct Webhook {
    id: WebhookId,
    url: AbsoluteUrl,
    key: SigningKey,
    events: Vec<Event>,
}

impl Webhook {
    /// A webhook `id` that sends `events` to `url`, signed with `key`.
    pub fn new(id: WebhookId, url: AbsoluteUrl, key: SigningKey, events: Vec<Event>) -> Self {
        Self {
            id,
            url,
            key,
            events,
        }
    }

    /// Its identifier.
    pub fn id(&self) -> WebhookId {
        self.id
    }

    /// Where deliveries go.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// The key deliveries are signed with.
    pub fn key(&self) -> &SigningKey {
        &self.key
    }

    /// Whether it asked for `event`.
    pub fn wants(&self, event: Event) -> bool {
        self.events.contains(&event)
    }
}
