//! A value that must never reach a log or a screen.

use std::fmt;

/// Wraps a token, key or password so it cannot be printed by accident
/// (RULES.md BL-RUN-5).
///
/// `Debug` and `Display` print `[redacted]`, so a secret inside a logged struct
/// or an error stays hidden. The value comes out only through
/// [`Secret::expose`], which makes every read easy to find. `Secret` is not
/// `Clone`, `PartialEq` or `Serialize`: copying, comparing and storing a secret
/// are choices to make on purpose, and tokens are compared in constant time
/// (BL-SEC-3).
///
/// # Examples
///
/// ```
/// use byline_core::Secret;
///
/// let token = Secret::new(String::from("generated-for-this-example"));
/// assert_eq!(format!("{token:?}"), "[redacted]");
/// assert_eq!(token.expose(), "generated-for-this-example");
/// ```
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps `value`.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// The value itself. Every call is a place where the secret leaves its
    /// wrapper, so keep the borrow short and never log it.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}
