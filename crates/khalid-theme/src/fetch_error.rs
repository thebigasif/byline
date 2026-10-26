//! Why a site's home page could not be read.

use khalid_outbound::OutboundError;

/// Why theme sync could not read a site's home page. The caller knows the
/// address it asked for, so the error does not repeat it. A stylesheet that
/// fails does not stop the sync; it is listed in
/// [`SiteSource::skipped`](crate::SiteSource::skipped) instead.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FetchError {
    /// The request did not complete: the name does not resolve, the address
    /// is private, or a time or size limit was passed.
    #[error("could not read the home page")]
    Unreachable {
        /// What the outbound client reported. `OutboundError::Blocked` means
        /// the address is private.
        #[source]
        source: OutboundError,
    },
    /// The server answered with a status other than 2xx.
    #[error("the home page answered with status {status}")]
    Status {
        /// The HTTP status code.
        status: u16,
    },
    /// The home page is larger than theme sync reads.
    #[error("the home page is larger than {limit} bytes")]
    TooLarge {
        /// The cap, in bytes.
        limit: usize,
    },
}
