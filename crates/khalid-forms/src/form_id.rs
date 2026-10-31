//! The name a form's answers are kept under.

use std::fmt;

/// The longest form id kept, in bytes.
const LONGEST: usize = 64;

/// A form's id, such as `contact`: lowercase letters, digits and `-`, as
/// the form block allows.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormId(String);

/// Why a text is not a form id.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a form id is 1 to 64 lowercase letters, digits and -")]
pub struct FormIdError;

impl FormId {
    /// The form id `text`.
    ///
    /// # Errors
    ///
    /// [`FormIdError`] when `text` is empty, longer than 64 bytes, or has
    /// anything but lowercase letters, digits and `-`.
    pub fn try_new(text: &str) -> Result<Self, FormIdError> {
        let plain = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if text.is_empty() || text.len() > LONGEST || !text.bytes().all(plain) {
            return Err(FormIdError);
        }
        Ok(Self(text.to_owned()))
    }

    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FormId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_block_s_ids_are_form_ids() {
        assert_eq!(FormId::try_new("contact-2").unwrap().as_str(), "contact-2");
        for wrong in ["", "Contact", "a_b", "a/b", "../x", &"a".repeat(65)] {
            assert_eq!(FormId::try_new(wrong), Err(FormIdError), "{wrong}");
        }
    }
}
