//! Reads a `filter` string into a [`Filter`].

use std::fmt;
use std::str::FromStr;

use super::filter::{Condition, Filter, Literal, Predicate};

/// The longest filter read, in characters: far above real use, cheap to parse.
const MAX_CHARS: usize = 2048;

impl FromStr for Filter {
    type Err = ParseFilterError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() > MAX_CHARS {
            return Err(ParseFilterError::at(MAX_CHARS, FilterErrorKind::TooLong));
        }
        if chars.is_empty() {
            return Err(ParseFilterError::at(0, FilterErrorKind::Empty));
        }
        let mut parser = Parser { chars, pos: 0 };
        let filter = parser.condition()?;
        match parser.peek() {
            Some(c) => Err(parser.error(FilterErrorKind::Unexpected(c))),
            None => Ok(filter),
        }
    }
}

/// A recursive-descent reader over the characters of one filter.
struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, want: char) -> bool {
        let found = self.peek() == Some(want);
        if found {
            self.pos += 1;
        }
        found
    }

    fn error(&self, kind: FilterErrorKind) -> ParseFilterError {
        ParseFilterError::at(self.pos, kind)
    }

    /// `field ':' predicate`
    fn condition(&mut self) -> Result<Filter, ParseFilterError> {
        let start = self.pos;
        let mut field = String::new();
        while let Some(c) = self
            .peek()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.'))
        {
            field.push(c);
            self.pos += 1;
        }
        if !field.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            return Err(ParseFilterError::at(start, FilterErrorKind::ExpectedField));
        }
        if !self.eat(':') {
            return Err(self.error(FilterErrorKind::ExpectedColon));
        }
        let predicate = if self.eat('-') {
            Predicate::IsNot(self.literal()?)
        } else {
            Predicate::Is(self.literal()?)
        };
        Ok(Filter::Condition(Condition::new(&field, predicate)))
    }

    /// A quoted string, or a bare word read as a keyword, number or text.
    fn literal(&mut self) -> Result<Literal, ParseFilterError> {
        let start = self.pos;
        if self.eat('\'') {
            return self.quoted(start).map(Literal::Text);
        }
        let mut word = String::new();
        while let Some(c) = self
            .peek()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            word.push(c);
            self.pos += 1;
        }
        Ok(match word.as_str() {
            "" => return Err(self.error(FilterErrorKind::ExpectedValue)),
            "true" => Literal::Bool(true),
            "false" => Literal::Bool(false),
            "null" => Literal::Null,
            _ => word.parse().map_or(Literal::Text(word), Literal::Integer),
        })
    }

    /// The rest of a single-quoted string; `\'` and `\\` are escapes.
    fn quoted(&mut self, start: usize) -> Result<String, ParseFilterError> {
        let unclosed = || ParseFilterError::at(start, FilterErrorKind::UnclosedQuote);
        let mut text = String::new();
        loop {
            let c = self.peek().ok_or_else(unclosed)?;
            self.pos += 1;
            match c {
                '\'' => return Ok(text),
                '\\' => {
                    text.push(self.peek().ok_or_else(unclosed)?);
                    self.pos += 1;
                }
                _ => text.push(c),
            }
        }
    }
}

/// A filter that could not be read, with the character where reading
/// stopped. `Display` reads "the filter needs a value at character 5",
/// counting characters from one for people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFilterError {
    position: usize,
    kind: FilterErrorKind,
}

impl fmt::Display for ParseFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the filter {}", self.kind)?;
        match self.kind {
            FilterErrorKind::Empty | FilterErrorKind::TooLong => Ok(()),
            _ => write!(f, " at character {}", self.position + 1),
        }
    }
}

impl std::error::Error for ParseFilterError {}

impl ParseFilterError {
    fn at(position: usize, kind: FilterErrorKind) -> Self {
        Self { position, kind }
    }

    /// Where reading stopped, counted in characters from zero.
    pub fn position(&self) -> usize {
        self.position
    }

    /// What was wrong.
    pub fn kind(&self) -> &FilterErrorKind {
        &self.kind
    }
}

/// What was wrong with a filter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterErrorKind {
    /// Nothing was given.
    Empty,
    /// Longer than 2,048 characters.
    TooLong,
    /// A field name was expected, such as `tag`.
    ExpectedField,
    /// A colon was expected after the field name.
    ExpectedColon,
    /// A value was expected.
    ExpectedValue,
    /// A quote was opened and never closed.
    UnclosedQuote,
    /// A character that does not belong here.
    Unexpected(char),
}

impl fmt::Display for FilterErrorKind {
    /// The phrase after "the filter".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "is empty",
            Self::TooLong => "is longer than 2048 characters",
            Self::ExpectedField => "needs a field name",
            Self::ExpectedColon => "needs a colon after the field name",
            Self::ExpectedValue => "needs a value",
            Self::UnclosedQuote => "has a quote that is not closed",
            Self::Unexpected(c) => return write!(f, "has an unexpected '{c}'"),
        })
    }
}
