//! What can be wrong with one field of a comment.

/// One field of a comment that cannot be kept as it was sent.
///
/// The binary turns each into a sentence in the reader's language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum FieldProblem {
    /// The name is empty.
    #[error("the name is empty")]
    NameMissing,
    /// The name is longer than [`AuthorName::MAX`](crate::AuthorName::MAX).
    #[error("the name is too long")]
    NameTooLong,
    /// The email address is not one.
    #[error("the email address is not valid")]
    EmailInvalid,
    /// The comment is empty.
    #[error("the comment is empty")]
    TextMissing,
    /// The comment is longer than [`CommentText::MAX`](crate::CommentText::MAX).
    #[error("the comment is too long")]
    TextTooLong,
    /// The comment answered is not a number.
    #[error("the comment answered is not valid")]
    ParentInvalid,
}
