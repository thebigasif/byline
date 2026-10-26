//! Serialising the shared identifier types as their text.

use std::fmt::Display;

/// Writes a value through its `Display`, so templates see `de` for a
/// [`khalid_core::Locale`] and the address for a [`khalid_core::AbsoluteUrl`].
pub(crate) fn display<T: Display, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

/// Like [`display`], for a value that may be absent.
#[expect(
    clippy::ref_option,
    reason = "serde's serialize_with hands over a reference to the field"
)]
pub(crate) fn display_option<T: Display, S: serde::Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.collect_str(value),
        None => serializer.serialize_none(),
    }
}
