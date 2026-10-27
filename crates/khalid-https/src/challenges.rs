//! The answers to the checks Let's Encrypt makes before it issues a
//! certificate: a file on port 80 (HTTP-01).

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// The answers to the checks Let's Encrypt is making right now, read by
/// the port 80 router while an order is open.
///
/// It is cheap to clone, and every clone sees the same answers.
#[derive(Debug, Clone, Default)]
pub struct Challenges {
    answers: Arc<RwLock<Answers>>,
}

/// The answers, by token for HTTP-01.
#[derive(Debug, Default)]
struct Answers {
    http: BTreeMap<String, String>,
}

impl Challenges {
    /// No answers yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The text to answer `/.well-known/acme-challenge/<token>` with, if
    /// that check is open.
    pub fn http_answer(&self, token: &str) -> Option<String> {
        self.read().http.get(token).cloned()
    }

    /// Answers the HTTP-01 check for `token` with `key_authorization`.
    pub fn answer_http(&self, token: &str, key_authorization: &str) {
        self.write()
            .http
            .insert(token.to_owned(), key_authorization.to_owned());
    }

    /// Forgets every answer, once an order is done.
    pub fn clear(&self) {
        let mut answers = self.write();
        answers.http.clear();
    }

    fn read(&self) -> RwLockReadGuard<'_, Answers> {
        self.answers.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, Answers> {
        self.answers.write().unwrap_or_else(PoisonError::into_inner)
    }
}
