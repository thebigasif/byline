//! Reads a `filter` string into a [`Filter`].

use std::fmt;
use std::str::FromStr;

use super::filter::{Condition, Filter, Literal, Predicate};

/// The longest filter read, in characters: far above real use, cheap to parse.
const MAX_CHARS: usize = 2048;

/// How deep brackets may nest. Ten is more than any real filter needs and
/// keeps recursion shallow.
const MAX_DEPTH: usize = 10;

/// How many conditions one filter may hold.
const MAX_CONDITIONS: usize = 100;

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
        let mut parser = Parser {
            chars,
            pos: 0,
            depth: 0,
            conditions: 0,
        };
        let filter = parser.or()?;
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
    depth: usize,
    conditions: usize,
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

    /// `and (',' and)*`
    fn or(&mut self) -> Result<Filter, ParseFilterError> {
        let mut parts = vec![self.and()?];
        while self.eat(',') {
            parts.push(self.and()?);
        }
        Ok(join(parts, Filter::Or))
    }

    /// `term ('+' term)*`
    fn and(&mut self) -> Result<Filter, ParseFilterError> {
        let mut parts = vec![self.term()?];
        while self.eat('+') {
            parts.push(self.term()?);
        }
        Ok(join(parts, Filter::And))
    }

    /// `'(' or ')' | condition`
    fn term(&mut self) -> Result<Filter, ParseFilterError> {
        if !self.eat('(') {
            return self.condition();
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error(FilterErrorKind::TooDeep));
        }
        let inner = self.or()?;
        if !self.eat(')') {
            return Err(self.error(FilterErrorKind::UnclosedBracket));
        }
        self.depth -= 1;
        Ok(inner)
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
        self.conditions += 1;
        if self.conditions > MAX_CONDITIONS {
            return Err(self.error(FilterErrorKind::TooManyConditions));
        }
        let predicate = self.predicate()?;
        Ok(Filter::Condition(Condition::new(&field, predicate)))
    }

    fn predicate(&mut self) -> Result<Predicate, ParseFilterError> {
        if self.eat('-') {
            if self.peek() == Some('[') {
                return Ok(Predicate::NotIn(self.list()?));
            }
            if self.eat('~') {
                return Ok(Predicate::NotContains(self.text()?));
            }
            return Ok(Predicate::IsNot(self.literal()?));
        }
        if self.peek() == Some('[') {
            return Ok(Predicate::In(self.list()?));
        }
        if self.eat('>') {
            return Ok(if self.eat('=') {
                Predicate::GreaterOrEqual(self.literal()?)
            } else {
                Predicate::Greater(self.literal()?)
            });
        }
        if self.eat('<') {
            return Ok(if self.eat('=') {
                Predicate::LessOrEqual(self.literal()?)
            } else {
                Predicate::Less(self.literal()?)
            });
        }
        if self.eat('~') {
            return Ok(if self.eat('^') {
                Predicate::StartsWith(self.text()?)
            } else if self.eat('$') {
                Predicate::EndsWith(self.text()?)
            } else {
                Predicate::Contains(self.text()?)
            });
        }
        Ok(Predicate::Is(self.literal()?))
    }

    /// `'[' literal (',' literal)* ']'`
    fn list(&mut self) -> Result<Vec<Literal>, ParseFilterError> {
        self.eat('[');
        let mut values = Vec::new();
        loop {
            values.push(self.literal()?);
            if self.eat(']') {
                return Ok(values);
            }
            if !self.eat(',') {
                return Err(match self.peek() {
                    None => self.error(FilterErrorKind::UnclosedBracket),
                    Some(c) => self.error(FilterErrorKind::Unexpected(c)),
                });
            }
        }
    }

    /// A literal that must be text, for the matching operators.
    fn text(&mut self) -> Result<String, ParseFilterError> {
        let start = self.pos;
        match self.literal()? {
            Literal::Text(text) => Ok(text),
            _ => Err(ParseFilterError::at(start, FilterErrorKind::ExpectedText)),
        }
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

/// One part stands alone; two or more are joined.
fn join(mut parts: Vec<Filter>, wrap: fn(Vec<Filter>) -> Filter) -> Filter {
    match parts.pop() {
        Some(only) if parts.is_empty() => only,
        Some(last) => {
            parts.push(last);
            wrap(parts)
        }
        None => wrap(parts),
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
    /// Brackets nested more than ten deep.
    TooDeep,
    /// More than 100 conditions.
    TooManyConditions,
    /// A field name was expected, such as `tag`.
    ExpectedField,
    /// A colon was expected after the field name.
    ExpectedColon,
    /// A value was expected.
    ExpectedValue,
    /// Quoted text was expected after `~`, `~^` or `~$`.
    ExpectedText,
    /// A quote was opened and never closed.
    UnclosedQuote,
    /// A bracket was opened and never closed.
    UnclosedBracket,
    /// A character that does not belong here.
    Unexpected(char),
}

impl fmt::Display for FilterErrorKind {
    /// The phrase after "the filter".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "is empty",
            Self::TooLong => "is longer than 2048 characters",
            Self::TooDeep => "nests brackets more than 10 deep",
            Self::TooManyConditions => "has more than 100 conditions",
            Self::ExpectedField => "needs a field name",
            Self::ExpectedColon => "needs a colon after the field name",
            Self::ExpectedValue => "needs a value",
            Self::ExpectedText => "needs quoted text",
            Self::UnclosedQuote => "has a quote that is not closed",
            Self::UnclosedBracket => "has a bracket that is not closed",
            Self::Unexpected(c) => return write!(f, "has an unexpected '{c}'"),
        })
    }
}
