//! One email sent to a segment, such as a post (task 5.8), and what the
//! store keeps about it.

/// What one email to a newsletter says, and the key that stops it going
/// out twice. A post's key names the post, so an edit never sends it
/// again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    key: String,
    subject: String,
    text: String,
}

impl Issue {
    /// The email `subject` with `text`, sent at most once under `key`,
    /// such as `post:en/choosing-anchor-text`.
    pub fn new(key: String, subject: String, text: String) -> Self {
        Self { key, subject, text }
    }

    /// The key it is sent once under.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The subject line.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The text, before the footer every subscriber's copy gets.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Where the site can be reached by post, which every newsletter email
/// ends with. Sending takes one, so nothing goes out without it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostalAddress(String);

impl PostalAddress {
    /// The address as `[mail]` holds it, or `None` when it is blank.
    pub fn new(text: &str) -> Option<Self> {
        let text = text.trim();
        (!text.is_empty()).then(|| Self(text.to_owned()))
    }

    /// The address as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One email sent to a newsletter, as the store numbers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SendId(i64);

impl SendId {
    /// The send with row number `row`.
    pub fn from_row(row: i64) -> Self {
        Self(row)
    }

    /// The row number to bind in a query.
    pub fn as_row(self) -> i64 {
        self.0
    }
}

/// How many copies of one email the transport took, and how many it
/// refused. Totals only, never who.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendTotals {
    /// Copies the transport took.
    pub sent: u32,
    /// Copies the transport refused or could not take.
    pub failed: u32,
}

/// What sending an issue did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sent {
    /// It went out now, with these totals.
    Now(SendTotals),
    /// It went out before under the same key, so nothing was sent.
    Before,
}
