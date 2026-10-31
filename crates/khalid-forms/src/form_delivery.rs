//! What happens to a form's answers once they are kept.

use std::fmt;

use chrono::{DateTime, Utc};

use crate::Submission;

/// Sends each kept answer on. The binary emails it to the site's owner and
/// fires the `form.submitted` webhook (G40).
pub trait FormDelivery: fmt::Debug + Send + Sync {
    /// Sends `submission`, kept `at`, on. It returns at once and sends on a
    /// task of its own, so the reader's thank-you page never waits for it.
    /// A failure is the delivery's to log or retry: the answer is kept.
    fn kept(&self, submission: &Submission, at: DateTime<Utc>);
}
