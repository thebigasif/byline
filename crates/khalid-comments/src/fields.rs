//! The fields a reader fills in, each checked once when it is read.

use crate::FieldProblem;

/// The name shown with a comment: one line, never empty.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorName(String);

impl AuthorName {
    /// The most letters a name may have.
    pub const MAX: usize = 80;

    /// The name in `raw`, with control characters dropped and each run of
    /// spaces and line breaks made one space.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::NameMissing`] when nothing is left, and
    /// [`FieldProblem::NameTooLong`] when more than [`Self::MAX`] letters are.
    pub fn parse(raw: &str) -> Result<Self, FieldProblem> {
        let words: Vec<String> = raw
            .split_whitespace()
            .map(|word| word.chars().filter(|c| !c.is_control()).collect())
            .filter(|word: &String| !word.is_empty())
            .collect();
        let name = words.join(" ");
        match name.chars().count() {
            0 => Err(FieldProblem::NameMissing),
            n if n > Self::MAX => Err(FieldProblem::NameTooLong),
            _ => Ok(Self(name)),
        }
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a comment says, as plain text with `\n` between lines. Never
/// empty, and never HTML: templates escape it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommentText(String);

impl CommentText {
    /// The most letters a comment may have.
    pub const MAX: usize = 5000;

    /// The text in `raw`, with Windows line ends made `\n`, control
    /// characters other than line breaks and tabs dropped, and the space
    /// around it trimmed.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::TextMissing`] when nothing is left, and
    /// [`FieldProblem::TextTooLong`] when more than [`Self::MAX`] letters are.
    pub fn parse(raw: &str) -> Result<Self, FieldProblem> {
        let lines = raw.replace("\r\n", "\n").replace('\r', "\n");
        let kept: String = lines
            .chars()
            .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
            .collect();
        let text = kept.trim();
        match text.chars().count() {
            0 => Err(FieldProblem::TextMissing),
            n if n > Self::MAX => Err(FieldProblem::TextTooLong),
            _ => Ok(Self(text.to_owned())),
        }
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A reader's email address. Kept so the site can write back, never shown.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReaderEmail(String);

impl ReaderEmail {
    /// The most bytes an address may have (RFC 5321).
    const MAX: usize = 254;

    /// The address in `raw`, trimmed: one `@` with a name before it and a
    /// domain with a dot after it, and no spaces. Whether it receives mail
    /// is not checked.
    ///
    /// # Errors
    ///
    /// [`FieldProblem::EmailInvalid`] when `raw` is not shaped like an address.
    pub fn parse(raw: &str) -> Result<Self, FieldProblem> {
        let address = raw.trim();
        let Some((name, domain)) = address.split_once('@') else {
            return Err(FieldProblem::EmailInvalid);
        };
        let shaped = !name.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains('@')
            && address.len() <= Self::MAX
            && !address.chars().any(|c| c.is_whitespace() || c.is_control());
        if shaped {
            Ok(Self(address.to_owned()))
        } else {
            Err(FieldProblem::EmailInvalid)
        }
    }

    /// The address.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
