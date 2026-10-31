//! [`SearchConsoleError`]: why Google gave no answer.

use khalid_outbound::OutboundError;

/// Why a request to Google did not give an answer. No variant holds a
/// secret or a token: they travel in request bodies and headers, never in
/// an address.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SearchConsoleError {
    /// Google no longer accepts the refresh token: the owner took the
    /// permission back, or it lapsed.
    #[error("Google no longer accepts the refresh token")]
    TokenRefused,
    /// Google does not know the OAuth client, or its secret is wrong.
    #[error("Google does not know the OAuth client or its secret")]
    ClientRefused,
    /// The signed-in account may not read the property.
    #[error("the Google account may not read this Search Console property")]
    NotAllowed,
    /// The day's or the minute's quota is spent.
    #[error("Google's quota for these requests is spent for now")]
    Quota,
    /// Google refused the request for another reason.
    #[error("Google answered {status}")]
    Refused {
        /// The HTTP status.
        status: u16,
    },
    /// The request did not complete.
    #[error("the request to Google did not complete")]
    Request {
        /// What the outbound client reported.
        #[source]
        source: OutboundError,
    },
    /// The answer is not the API's JSON.
    #[error("Google answered with something that is not its usual JSON")]
    Answer,
    /// An address for the request could not be made. Nothing was sent.
    #[error("the address of the request to Google could not be made")]
    Address,
}

impl SearchConsoleError {
    /// The error for a refusal with HTTP `status` and Google's JSON `body`,
    /// either OAuth's `{"error": "invalid_grant"}` or the APIs'
    /// `{"error": {"status": "PERMISSION_DENIED"}}`.
    pub fn refused(status: u16, body: &[u8]) -> Self {
        let fault: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
        let error = fault.get("error");
        let name = error
            .and_then(|e| e.as_str().or_else(|| e.get("status")?.as_str()))
            .unwrap_or_default();
        match (status, name) {
            (_, "invalid_grant") => Self::TokenRefused,
            (_, "invalid_client" | "unauthorized_client") => Self::ClientRefused,
            (403, _) | (_, "PERMISSION_DENIED") => Self::NotAllowed,
            (429, _) | (_, "RESOURCE_EXHAUSTED") => Self::Quota,
            _ => Self::Refused { status },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_is_named_from_either_shape_of_error() {
        let api = br#"{"error": {"code": 403, "status": "PERMISSION_DENIED"}}"#;
        assert!(matches!(
            SearchConsoleError::refused(403, api),
            SearchConsoleError::NotAllowed
        ));
        let quota = br#"{"error": {"code": 429, "status": "RESOURCE_EXHAUSTED"}}"#;
        assert!(matches!(
            SearchConsoleError::refused(429, quota),
            SearchConsoleError::Quota
        ));
        assert!(matches!(
            SearchConsoleError::refused(400, br#"{"error": "unauthorized_client"}"#),
            SearchConsoleError::ClientRefused
        ));
        let odd = SearchConsoleError::refused(500, b"<html></html>").to_string();
        assert_eq!(odd, "Google answered 500");
    }
}
