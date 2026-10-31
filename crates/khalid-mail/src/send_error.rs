//! Why an email did not leave.

use std::error::Error;
use std::io;

use khalid_outbound::{OutboundError, ResolveError};

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
    /// The SMTP server's public host name could not be found, or the
    /// private-network guard refused an address it resolves to.
    #[error("the SMTP server could not be looked up")]
    Resolve(#[source] ResolveError),
    /// The SMTP host marked `private_host` could not be found.
    #[error("the SMTP server on your own network could not be found")]
    Lookup(#[source] io::Error),
    /// An address could not be written into the email's headers.
    #[error("an address in the email is not one SMTP can carry")]
    Address(#[source] Box<dyn Error + Send + Sync>),
    /// The email could not be written as an SMTP message.
    #[error("the email could not be written as an SMTP message")]
    Message(#[source] Box<dyn Error + Send + Sync>),
    /// The SMTP server could not be reached, refused to talk, refused the
    /// password, or did not take the email.
    #[error("the SMTP server did not take the email")]
    Smtp(#[source] Box<dyn Error + Send + Sync>),
}
