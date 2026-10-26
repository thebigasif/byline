//! Retrying webhook deliveries that are due, on a timer.

use std::sync::Arc;
use std::time::Duration;

use khalid_webhooks::{DeliveryStore, Dispatcher, Webhook};
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

use crate::Background;

/// Every `every`, sends again each delivery in the dispatcher's log that is
/// due, to the webhooks in `webhooks`, through the dispatcher's outbound
/// client (RULES.md BL-RUN-8). The dispatcher reads the time from its own
/// clock.
///
/// A log that cannot be read is logged and tried again on the next round;
/// a receiver that fails is the dispatcher's to schedule again. Stop it
/// before closing the database, so no round is cut off (BL-RUN-10).
pub fn retry_deliveries<S>(
    dispatcher: Arc<Dispatcher<S>>,
    webhooks: Vec<Webhook>,
    every: Duration,
) -> Background
where
    S: DeliveryStore + Send + Sync + 'static,
{
    Background::spawn(move |stopped| retry_until_stopped(dispatcher, webhooks, every, stopped))
}

async fn retry_until_stopped<S: DeliveryStore + Send + Sync + 'static>(
    dispatcher: Arc<Dispatcher<S>>,
    webhooks: Vec<Webhook>,
    every: Duration,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut ticks = tokio::time::interval(every);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            _ = ticks.tick() => {}
        }
        match dispatcher.retry_due(&webhooks).await {
            Ok(retried) if retried.is_empty() => {}
            Ok(retried) => tracing::info!(count = retried.len(), "retried webhook deliveries"),
            Err(error) => tracing::warn!(
                ?error,
                "the webhook delivery log could not be read, so retries wait for the next round"
            ),
        }
    }
}
