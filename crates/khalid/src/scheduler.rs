//! The scheduler in `khalid serve`: every few seconds it asks the publish
//! door to act on each post whose publish or unpublish time has come. It
//! never publishes on its own, so every post it takes live or down is
//! announced like any other (CTO answers 2026-10-27, section 4).

use std::sync::Arc;
use std::time::Duration;

use khalid_api::publish_door::{PublishDoor, Ran};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

use crate::alerts::Alerting;

/// How often the schedules are looked at, so a post goes live at most this
/// long after its time.
const EVERY: Duration = Duration::from_secs(5);

/// The running scheduler, stopped before the database closes.
#[derive(Debug)]
pub(crate) struct Scheduling {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl Scheduling {
    /// Stops the scheduler once the round in progress has finished
    /// (RULES.md BL-RUN-10).
    pub(crate) async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Starts the scheduler on `door`, raising each post the linter refuses
/// through `alerting`. Must be called inside the tokio
/// runtime. The first round runs at once, so a time that passed while the
/// server was stopped is acted on at start.
pub(crate) fn start(door: Arc<PublishDoor>, alerting: Arc<Alerting>) -> Scheduling {
    let (stop, mut stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut ticks = tokio::time::interval(EVERY);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = &mut stopped => return,
                _ = ticks.tick() => {}
            }
            match door.run_due().await {
                Ok(ran) => {
                    for one in &ran {
                        report(one);
                        if let Ran::Refused(id) = one {
                            alerting.refused(id).await;
                        }
                    }
                }
                Err(error) => tracing::warn!(
                    ?error,
                    "the schedules could not be read, so they are looked at again next round"
                ),
            }
        }
    });
    Scheduling { stop, task }
}

/// Logs what the scheduler did about one post.
fn report(ran: &Ran) {
    if let Ran::Refused(id) = ran {
        tracing::warn!(
            id,
            "the linter refused a scheduled post, so it stays a draft"
        );
    } else {
        tracing::info!(?ran, "the scheduler acted on a post whose time had come");
    }
}
