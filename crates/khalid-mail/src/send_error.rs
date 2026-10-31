//! Why an email did not leave.

use khalid_outbound::OutboundError;

/// Why a transport did not take an email. It never holds the email's text,
/// so it is safe to log.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SendError {
    /// The mail service answered with a status other than 2xx, such as 401
    /// for a wrong key or 422 for a sender it has not verified.
    #[error("the mail service refused the email with status {status}")]
    Refused {
        /// The HTTP status it answered.
        status: u16,
    },
    /// The mail service could not be reached, or the private-network guard
    /// refused its address.
    #[error("the mail service could not be reached")]
    Unreachable(#[source] OutboundError),
    /// The email could not be written as the service's JSON.
    #[error("the email could not be written for the mail service")]
    Encode(#[source] serde_json::Error),
}
