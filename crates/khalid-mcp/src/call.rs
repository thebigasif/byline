//! One request a tool sends to the API, before it is signed with a key.

use axum::http::Method;

/// One request, with the key it carries, its extra headers and its body.
#[derive(Debug)]
pub(crate) struct Call {
    pub(crate) method: Method,
    pub(crate) uri: String,
    pub(crate) key: Key,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Payload>,
}

/// A request body and its media type.
#[derive(Debug)]
pub(crate) struct Payload {
    pub(crate) media: String,
    pub(crate) bytes: Vec<u8>,
}

/// Which key a request carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    /// The server's own content key, for the Content API.
    Reader,
    /// The integration key or staff token the server acts with.
    Writer,
}
