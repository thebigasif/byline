//! The `comment.added` webhook (plan section 4.10, KH-API-3).

use std::sync::Arc;

use khalid_api::resources::AdminComment;
use khalid_webhooks::{DeliveryStore, Dispatcher, Event, Webhook};
use serde::Serialize;

use crate::{Audience, Comment, CommentEvents, api_shape};

/// Sends `comment.added` to every webhook that asked for it, each time a
/// comment is stored. The comment goes in API shape without the reader's
/// email. Sending runs on a task of its own, so a slow receiver never
/// holds up the reader; the dispatcher logs and retries failures.
#[derive(Debug)]
pub struct WebhookEvents<S> {
    dispatcher: Arc<Dispatcher<S>>,
    webhooks: Arc<[Webhook]>,
}

/// The resource a `comment.added` delivery carries.
#[derive(Debug, Serialize)]
struct Added {
    comment: AdminComment,
}

impl<S> WebhookEvents<S> {
    /// Sends through `dispatcher` to `webhooks`.
    pub fn new(dispatcher: Arc<Dispatcher<S>>, webhooks: Vec<Webhook>) -> Self {
        Self {
            dispatcher,
            webhooks: webhooks.into(),
        }
    }
}

impl<S: DeliveryStore + std::fmt::Debug + 'static> CommentEvents for WebhookEvents<S> {
    fn added(&self, comment: &Comment) {
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            tracing::warn!("comment.added was not sent: no runtime is running");
            return;
        };
        let added = Added {
            comment: api_shape(comment, Audience::Integrations),
        };
        let dispatcher = Arc::clone(&self.dispatcher);
        let webhooks = Arc::clone(&self.webhooks);
        runtime.spawn(async move {
            let sent = dispatcher.publish(&webhooks, Event::CommentAdded, &added);
            if let Err(error) = sent.await {
                tracing::warn!(%error, "comment.added could not be logged");
            }
        });
    }
}
