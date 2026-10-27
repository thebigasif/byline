//! [`IndexNow::send`]: changed addresses sent through [`Outbound`], in
//! batches, with retries.

use std::time::Duration;

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Fetched, Outbound, OutboundError};

use crate::{IndexNow, SeoError, Submission};

/// The pauses before each retry: a short one for a blip, longer ones for a
/// search engine that is busy or down. No random jitter, so runs are
/// predictable (BL-TST-2).
const DEFAULT_PAUSES: [Duration; 3] = [
    Duration::from_secs(10),
    Duration::from_mins(1),
    Duration::from_mins(5),
];

/// The header every request carries.
const JSON: (&str, &str) = ("Content-Type", "application/json; charset=utf-8");

/// How often a failed `IndexNow` request is tried again, and the pause
/// before each try. One attempt, plus one per pause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retries {
    pauses: Vec<Duration>,
}

impl Retries {
    /// Retries after each pause in `pauses`, in order. No pauses, no retries.
    pub fn new(pauses: Vec<Duration>) -> Self {
        Self { pauses }
    }

    /// The pauses before each retry, in order.
    pub fn pauses(&self) -> &[Duration] {
        &self.pauses
    }
}

impl Default for Retries {
    /// Three retries, after 10 seconds, 1 minute and 5 minutes.
    fn default() -> Self {
        Self::new(DEFAULT_PAUSES.to_vec())
    }
}

/// What [`IndexNow::send`] sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sent {
    requests: usize,
    urls: usize,
    attempts: usize,
}

impl Sent {
    /// The requests search engines accepted.
    pub fn requests(&self) -> usize {
        self.requests
    }

    /// The addresses those requests announced, each once.
    pub fn urls(&self) -> usize {
        self.urls
    }

    /// The attempts made in all, retries included.
    pub fn attempts(&self) -> usize {
        self.attempts
    }
}

/// Why changed addresses did not reach `IndexNow`.
///
/// Requests before the failing one were accepted; later ones were not sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SendIndexNowError {
    /// The addresses cannot be sent, so nothing was.
    #[error("the changed addresses cannot be sent to IndexNow")]
    Payload {
        /// Why the request could not be built.
        #[from]
        source: SeoError,
    },
    /// The search engine answered with a status other than success: 400
    /// for a bad request, 403 for a key it could not check, 422 for an
    /// address that is not the site's, 429 or 5xx when it stayed busy.
    #[error("IndexNow answered {status} after {attempts} attempts")]
    Status {
        /// The last HTTP status.
        status: u16,
        /// The attempts made for this request.
        attempts: usize,
    },
    /// The request did not complete: refused by the guard, timed out or
    /// unable to connect.
    #[error("the IndexNow request did not complete after {attempts} attempts")]
    Request {
        /// The attempts made for this request.
        attempts: usize,
        /// What the outbound client reported last.
        #[source]
        source: OutboundError,
    },
}

impl IndexNow {
    /// Tells search engines that `changed` addresses changed: each address
    /// once, 10,000 to a request, each request through `outbound` and tried
    /// again after each of `retries`' pauses while the failure may pass.
    ///
    /// A 429, a 5xx, a timeout or a failed connection is tried again. A 400,
    /// 403 or 422, or a request the guard refuses, is not, because it would
    /// fail the same way. Nothing changed sends nothing.
    ///
    /// # Errors
    ///
    /// [`SendIndexNowError::Payload`] before anything is sent when an
    /// address is outside the key file's folder, and
    /// [`SendIndexNowError::Status`] or [`SendIndexNowError::Request`] for
    /// the first request that still fails after its retries.
    pub async fn send(
        &self,
        outbound: &Outbound,
        retries: &Retries,
        changed: &[AbsoluteUrl],
    ) -> Result<Sent, SendIndexNowError> {
        let submissions = self.submissions(changed)?;
        let mut sent = Sent {
            requests: 0,
            urls: 0,
            attempts: 0,
        };
        for submission in &submissions {
            let attempts = self.deliver(outbound, retries, submission).await?;
            sent.requests = sent.requests.saturating_add(1);
            sent.urls = sent.urls.saturating_add(submission.urls().len());
            sent.attempts = sent.attempts.saturating_add(attempts);
        }
        Ok(sent)
    }

    /// Sends one request until it is accepted or no retry is left, and
    /// returns the attempts it took.
    async fn deliver(
        &self,
        outbound: &Outbound,
        retries: &Retries,
        submission: &Submission,
    ) -> Result<usize, SendIndexNowError> {
        let mut pauses = retries.pauses().iter();
        let mut attempts: usize = 0;
        loop {
            attempts = attempts.saturating_add(1);
            let answer = outbound
                .post(self.endpoint(), &[JSON], submission.body().to_vec())
                .await;
            let error = match answer {
                Ok(fetched) if is_success(&fetched) => return Ok(attempts),
                Ok(fetched) => SendIndexNowError::Status {
                    status: fetched.status(),
                    attempts,
                },
                Err(source) => SendIndexNowError::Request { attempts, source },
            };
            match pauses.next() {
                Some(pause) if may_pass(&error) => tokio::time::sleep(*pause).await,
                _ => return Err(error),
            }
        }
    }
}

/// Whether the search engine took the request: any 2xx, such as 200, or 202
/// for later.
fn is_success(fetched: &Fetched) -> bool {
    (200..300).contains(&fetched.status())
}

/// Whether trying the same request again may succeed.
fn may_pass(error: &SendIndexNowError) -> bool {
    match error {
        SendIndexNowError::Status { status, .. } => *status == 429 || *status >= 500,
        SendIndexNowError::Request { source, .. } => {
            matches!(source, OutboundError::TimedOut | OutboundError::Http { .. })
        }
        _ => false,
    }
}
