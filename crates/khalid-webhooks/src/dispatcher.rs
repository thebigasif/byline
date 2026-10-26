//! Sends deliveries through the outbound client and records each attempt.

use chrono::SecondsFormat;
use khalid_core::{AbsoluteUrl, Clock};
use khalid_outbound::Outbound;
use serde::Serialize;

use crate::{
    Attempt, Delivery, DeliveryStore, DeliveryStoreError, Event, NewDelivery, Outcome,
    RetrySchedule, SIGNATURE_HEADER, Webhook, sign,
};

/// The body every delivery carries: the resource in API shape plus
/// `event`, `occurred_at` and `site` (`docs/standards/api.md`).
#[derive(Serialize)]
struct Payload<'a, R> {
    event: Event,
    occurred_at: String,
    site: &'a str,
    #[serde(flatten)]
    resource: &'a R,
}

/// Announces events: one signed delivery per subscribed webhook, sent
/// through [`Outbound`] (RULES.md BL-RUN-8) and logged in the
/// [`DeliveryStore`]; failures are retried on the [`RetrySchedule`]. The
/// server runs it on its own task, so a slow receiver never delays a
/// request.
#[derive(Debug)]
pub struct Dispatcher<S> {
    outbound: Outbound,
    store: S,
    clock: Box<dyn Clock>,
    site: AbsoluteUrl,
    schedule: RetrySchedule,
}

impl<S: DeliveryStore> Dispatcher<S> {
    /// A dispatcher for the site at `site`, logging to `store`, with the
    /// default retry schedule.
    pub fn new(
        outbound: Outbound,
        store: S,
        clock: impl Clock + 'static,
        site: AbsoluteUrl,
    ) -> Self {
        Self {
            outbound,
            store,
            clock: Box::new(clock),
            site,
            schedule: RetrySchedule::default(),
        }
    }

    /// Sends `event` about `resource` to every webhook in `webhooks` that
    /// asked for it, once each, and returns the deliveries as logged.
    /// `resource` serialises as an object such as `{"post": {...}}`.
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError::Serialize`] when `resource` is not a JSON
    /// object, and [`DispatchError::Store`] when the log cannot be written.
    /// A receiver that fails is not an error: its delivery is logged and
    /// retried.
    pub async fn publish<R: Serialize>(
        &self,
        webhooks: &[Webhook],
        event: Event,
        resource: &R,
    ) -> Result<Vec<Delivery>, DispatchError> {
        let now = self.clock.now();
        let payload = Payload {
            event,
            occurred_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
            site: self.site.as_str(),
            resource,
        };
        let body =
            serde_json::to_vec(&payload).map_err(|source| DispatchError::Serialize { source })?;
        let mut sent = Vec::new();
        for webhook in webhooks.iter().filter(|webhook| webhook.wants(event)) {
            let new = NewDelivery::new(
                webhook.id(),
                event,
                webhook.url().clone(),
                body.clone(),
                now,
            );
            let delivery = self.store.insert(new).await?;
            sent.push(self.attempt(webhook, delivery).await?);
        }
        Ok(sent)
    }

    /// Tries again every delivery that is due, and returns them as logged.
    /// The server calls this on a timer. A delivery whose webhook is not in
    /// `webhooks` was removed with it, and is skipped.
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError::Store`] when the log cannot be read or
    /// written.
    pub async fn retry_due(&self, webhooks: &[Webhook]) -> Result<Vec<Delivery>, DispatchError> {
        let mut retried = Vec::new();
        for delivery in self.store.due(self.clock.now()).await? {
            let owner = webhooks.iter().find(|hook| hook.id() == delivery.webhook());
            if let Some(webhook) = owner {
                retried.push(self.attempt(webhook, delivery).await?);
            }
        }
        Ok(retried)
    }

    /// Signs the body afresh, sends it, and records what came back.
    async fn attempt(
        &self,
        webhook: &Webhook,
        mut delivery: Delivery,
    ) -> Result<Delivery, DispatchError> {
        let now = self.clock.now();
        let signature = sign(webhook.key(), now, delivery.body()).to_string();
        let id = delivery.id().get().to_string();
        let headers = [
            ("content-type", "application/json"),
            ("user-agent", "Khalid-Webhooks"),
            ("khalid-event", delivery.event().as_str()),
            ("khalid-delivery", id.as_str()),
            (SIGNATURE_HEADER, signature.as_str()),
        ];
        let sent = self
            .outbound
            .post(delivery.url(), &headers, delivery.body().to_vec())
            .await;
        let outcome = match sent {
            Ok(answer) => Outcome::Answered {
                status: answer.status(),
            },
            Err(error) => Outcome::Unreachable {
                reason: error.to_string(),
            },
        };
        delivery.record(Attempt::new(now, outcome), &self.schedule);
        self.store.update(&delivery).await?;
        Ok(delivery)
    }
}

/// Why events could not be dispatched.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DispatchError {
    /// The resource could not be written as a JSON object.
    #[error("the webhook body could not be written as JSON")]
    Serialize {
        /// What the serialiser reported.
        #[source]
        source: serde_json::Error,
    },
    /// The delivery log could not be read or written.
    #[error(transparent)]
    Store(#[from] DeliveryStoreError),
}
