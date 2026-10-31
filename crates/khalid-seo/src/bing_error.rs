//! [`BingError`]: why a Bing Webmaster Tools request gave no answer.

use khalid_outbound::OutboundError;

/// Why a Bing Webmaster Tools request did not give an answer. No variant
/// holds the request's address, because it carries the key.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BingError {
    /// The key cannot be one Bing issued. Nothing was sent.
    #[error("the Bing Webmaster Tools API key holds characters Bing's keys never have")]
    Key,
    /// The site's address could not be made into a request. Nothing was sent.
    #[error("the address of the Bing Webmaster Tools request could not be made")]
    Address,
    /// Bing does not know the key.
    #[error("Bing Webmaster Tools does not know the API key")]
    KeyRefused,
    /// The key's owner has not verified the site in Bing Webmaster Tools.
    #[error("the site is not verified in Bing Webmaster Tools for this API key")]
    NotVerified,
    /// Bing refused the request for another reason, such as too many
    /// requests.
    #[error("Bing Webmaster Tools answered {status}: {reason}")]
    Refused {
        /// The HTTP status.
        status: u16,
        /// Bing's one-word reason, such as `ThrottleUser`.
        reason: String,
    },
    /// The request did not complete.
    #[error("the request to Bing Webmaster Tools did not complete: {failure}")]
    Request {
        /// What went wrong, without the address.
        failure: RequestFailure,
    },
    /// The answer is not the API's JSON.
    #[error("Bing Webmaster Tools answered with something that is not its usual JSON")]
    Answer,
}

impl BingError {
    /// The longest word from Bing that an error repeats.
    const MAX_REASON: usize = 64;

    /// The error for a refusal with HTTP `status` and Bing's `message`, such
    /// as `InvalidApiKey`. A message that is not one short word, as Bing's
    /// error names are, is not repeated.
    pub fn refused(status: u16, message: &str) -> Self {
        match message {
            "InvalidApiKey" => Self::KeyRefused,
            "NotAuthorized" => Self::NotVerified,
            _ => {
                let word = !message.is_empty()
                    && message.len() <= Self::MAX_REASON
                    && message.chars().all(|c| c.is_ascii_alphanumeric());
                let reason = if word {
                    message
                } else {
                    "no reason Khalid can show"
                };
                Self::Refused {
                    status,
                    reason: reason.to_owned(),
                }
            }
        }
    }
}

/// Why a request did not complete, kept apart from the HTTP client's error,
/// whose text holds the address and so the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RequestFailure {
    /// The private-network guard refused where the name led.
    #[error("the address is not a public one")]
    Blocked,
    /// No answer came in time.
    #[error("it took longer than its time limit")]
    TimedOut,
    /// The answer was larger than the limit.
    #[error("the answer was too large")]
    TooLarge,
    /// Anything else, such as a name that cannot be found.
    #[error("the connection failed")]
    Connection,
}

impl From<&OutboundError> for RequestFailure {
    fn from(error: &OutboundError) -> Self {
        match error {
            OutboundError::Blocked { .. } => Self::Blocked,
            OutboundError::TimedOut => Self::TimedOut,
            OutboundError::TooLarge { .. } => Self::TooLarge,
            _ => Self::Connection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_is_named_and_repeats_only_a_short_word() {
        assert!(matches!(
            BingError::refused(400, "InvalidApiKey"),
            BingError::KeyRefused
        ));
        assert!(matches!(
            BingError::refused(400, "NotAuthorized"),
            BingError::NotVerified
        ));
        let throttled = BingError::refused(400, "ThrottleUser").to_string();
        assert_eq!(throttled, "Bing Webmaster Tools answered 400: ThrottleUser");
        for odd in ["", "two words", "<b>html</b>", &"a".repeat(65)] {
            let said = BingError::refused(403, odd).to_string();
            assert!(said.ends_with("403: no reason Khalid can show"), "{said}");
        }
    }

    #[test]
    fn a_failed_request_keeps_only_its_kind() {
        let limit = OutboundError::TooLarge { limit: 1 };
        assert_eq!(RequestFailure::from(&limit), RequestFailure::TooLarge);
        assert_eq!(
            RequestFailure::from(&OutboundError::TimedOut),
            RequestFailure::TimedOut
        );
        let error = BingError::Request {
            failure: RequestFailure::TimedOut,
        };
        assert_eq!(
            error.to_string(),
            "the request to Bing Webmaster Tools did not complete: it took longer than its time limit"
        );
    }
}
