//! An email address a reader typed, checked once at the edge.

use std::fmt;

/// The longest address SMTP carries, in bytes (RFC 5321, section 4.5.3.1).
const LONGEST: usize = 254;

/// The longest part before the `@`, in bytes.
const LONGEST_LOCAL: usize = 64;

/// The longest part of a domain between dots, in bytes.
const LONGEST_LABEL: usize = 63;

/// An address that can receive mail: one `@`, a plain part before it, and a
/// domain with at least one dot after it. Kept in lowercase, so the same
/// person typed twice is one subscriber.
///
/// Only ASCII is taken. A domain in another script is typed in its `xn--`
/// form, which every mail program shows on request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// Checks `text` as typed, without the spaces around it.
    ///
    /// # Errors
    ///
    /// [`ParseEmailError`] when `text` cannot be an address that receives
    /// mail. The error does not repeat `text`, so logging it is safe.
    pub fn parse(text: &str) -> Result<Self, ParseEmailError> {
        let text = text.trim().to_ascii_lowercase();
        let (local, domain) = text.split_once('@').ok_or(ParseEmailError)?;
        let fits = text.len() <= LONGEST && local.len() <= LONGEST_LOCAL;
        if fits && is_local(local) && is_domain(domain) {
            Ok(Self(text))
        } else {
            Err(ParseEmailError)
        }
    }

    /// The address as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether `local` is a plain part before the `@`: printable ASCII with no
/// space and none of the characters that need quoting.
fn is_local(local: &str) -> bool {
    !local.is_empty()
        && local
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !b"\"(),:;<>@[\\]".contains(&byte))
}

/// Whether `domain` is two or more labels of letters, digits and inner
/// hyphens, joined by dots.
fn is_domain(domain: &str) -> bool {
    let mut labels = 0_usize;
    for label in domain.split('.') {
        let inner = label.len() <= LONGEST_LABEL
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if label.is_empty() || !inner {
            return false;
        }
        labels = labels.saturating_add(1);
    }
    labels >= 2
}

/// Text that is not an email address that can receive mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this is not an email address that can receive mail")]
pub struct ParseEmailError;
