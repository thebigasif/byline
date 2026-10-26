//! A job the server runs beside the requests, and the switch that stops it.

use std::future::Future;

use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// A running background job, such as the file watch or the webhook retry
/// timer. Dropping it stops the job too; [`Background::stop`] also waits
/// until the job has finished its current round.
#[derive(Debug)]
pub struct Background {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl Background {
    /// Runs `job` on its own task. The job returns when the receiver it is
    /// given completes, which happens on [`Background::stop`] or a drop.
    /// Must be called inside the tokio runtime.
    pub(crate) fn spawn<F>(job: impl FnOnce(oneshot::Receiver<()>) -> F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(job(stopped));
        Self { stop, task }
    }

    /// Stops the job, and waits for a round in progress to finish, so the
    /// server can close the database after it (RULES.md BL-RUN-10).
    pub async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}
