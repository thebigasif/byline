//! Retrying webhook deliveries that are due, on a timer.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use khalid_webhooks::{DeliveryStore, Dispatcher, Webhook};
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

use crate::Background;

/// Every `every`, sends again each delivery in the dispatcher's log that is
/// due, to the webhooks `list` reads, through the dispatcher's outbound
/// client (RULES.md BL-RUN-8). The dispatcher reads the time from its own
/// clock.
///
/// `list` is read afresh each round, so a webhook added while the server
/// runs is retried, and one removed is left alone. A list or a log that
/// cannot be read is logged and tried again on the next round; a receiver
/// that fails is the dispatcher's to schedule again. Stop it before
/// closing the database, so no round is cut off (BL-RUN-10).
pub fn retry_deliveries<S, L, F, E>(
    dispatcher: Arc<Dispatcher<S>>,
    list: L,
    every: Duration,
) -> Background
where
    S: DeliveryStore + Send + Sync + 'static,
    L: Fn() -> F + Send + 'static,
    F: Future<Output = Result<Vec<Webhook>, E>> + Send + 'static,
    E: fmt::Display + 'static,
{
    Background::spawn(move |stopped| retry_until_stopped(dispatcher, list, every, stopped))
}

async fn retry_until_stopped<S, L, F, E>(
    dispatcher: Arc<Dispatcher<S>>,
    list: L,
    every: Duration,
    mut stopped: oneshot::Receiver<()>,
) where
    S: DeliveryStore + Send + Sync + 'static,
    L: Fn() -> F + Send + 'static,
    F: Future<Output = Result<Vec<Webhook>, E>> + Send + 'static,
    E: fmt::Display + 'static,
{
    let mut ticks = tokio::time::interval(every);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            _ = ticks.tick() => {}
        }
        let webhooks = match list().await {
            Ok(webhooks) => webhooks,
            Err(error) => {
                let error = error.to_string();
                tracing::warn!(
                    %error,
                    "the webhook list could not be read, so retries wait for the next round"
                );
                continue;
            }
        };
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
