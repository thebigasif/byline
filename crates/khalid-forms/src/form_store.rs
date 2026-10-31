//! Where form answers are kept.

use std::error::Error;
use std::future::Future;

use chrono::{DateTime, Utc};

use crate::Submission;

/// The answers' storage. `khalid-db` implements it on SQLite.
pub trait FormStore: Send + Sync {
    /// Keeps `submission`, sent `at`.
    ///
    /// # Errors
    ///
    /// [`FormStoreError::Write`] when it cannot be written.
    fn insert(
        &self,
        submission: &Submission,
        at: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), FormStoreError>> + Send;
}

/// Why the storage failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormStoreError {
    /// Writing failed.
    #[error("the form's answers could not be written")]
    Write {
        /// What the storage reported.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}
