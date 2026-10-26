//! A parsed `filter`: conditions joined with and and or.

use std::fmt::{self, Write as _};

/// A `filter` in Ghost's syntax, parsed: `tag:news+featured:true`,
/// `tag:[seo,links]`, `tag:-draft`, `published_at:>'2026-01-01'`.
///
/// `+` (and) binds tighter than `,` (or), and brackets group. A list of
/// one is never wrapped, so `tag:news` is a bare [`Filter::Condition`].
/// Groups keep the shape they were written in.
///
/// # Examples
///
/// ```
/// use khalid_api::query::Filter;
///
/// let filter: Filter = "tag:news+featured:true".parse()?;
/// let Filter::And(parts) = filter else { return Err("not an and".into()) };
/// assert_eq!(parts.len(), 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Filter {
    /// Every part must hold. Two or more parts.
    And(Vec<Filter>),
    /// At least one part must hold. Two or more parts.
    Or(Vec<Filter>),
    /// One field tested one way.
    Condition(Condition),
}

/// One field and the test applied to it, such as `tag:-draft`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    field: String,
    predicate: Predicate,
}

impl Condition {
    /// `field` tested with `predicate`. Fields are checked when applied.
    pub fn new(field: &str, predicate: Predicate) -> Self {
        Self {
            field: field.to_owned(),
            predicate,
        }
    }

    /// The field name as written, such as `tag` or `authors.slug`.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The test.
    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }
}

/// How a field is tested.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Predicate {
    /// `field:value`
    Is(Literal),
    /// `field:-value`
    IsNot(Literal),
    /// `field:>value`
    Greater(Literal),
    /// `field:>=value`
    GreaterOrEqual(Literal),
    /// `field:<value`
    Less(Literal),
    /// `field:<=value`
    LessOrEqual(Literal),
    /// `field:[a,b]`
    In(Vec<Literal>),
    /// `field:-[a,b]`
    NotIn(Vec<Literal>),
    /// `field:~'text'`
    Contains(String),
    /// `field:-~'text'`
    NotContains(String),
    /// `field:~^'text'`
    StartsWith(String),
    /// `field:~$'text'`
    EndsWith(String),
}

/// A value in a filter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Literal {
    /// Text, quoted (`'2026-01-01'`) or a bare word (`news`).
    Text(String),
    /// A whole number written without quotes.
    Integer(i64),
    /// `true` or `false` without quotes.
    Bool(bool),
    /// `null` without quotes.
    Null,
}

/// Writes the canonical form, which parses back to the same filter: text
/// always quoted, and brackets only where a group changes the meaning or
/// was written inside its own kind.
impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Condition(condition) => condition.fmt(f),
            Self::And(parts) => join(f, parts, '+', |part| !matches!(part, Self::Condition(_))),
            Self::Or(parts) => join(f, parts, ',', |part| matches!(part, Self::Or(_))),
        }
    }
}

/// Writes `parts` with `separator`, bracketing those `grouped` picks.
fn join(
    f: &mut fmt::Formatter<'_>,
    parts: &[Filter],
    separator: char,
    grouped: fn(&Filter) -> bool,
) -> fmt::Result {
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            f.write_char(separator)?;
        }
        if grouped(part) {
            write!(f, "({part})")?;
        } else {
            write!(f, "{part}")?;
        }
    }
    Ok(())
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:", self.field)?;
        let (operator, value) = match &self.predicate {
            Predicate::Is(value) => ("", value),
            Predicate::IsNot(value) => ("-", value),
            Predicate::Greater(value) => (">", value),
            Predicate::GreaterOrEqual(value) => (">=", value),
            Predicate::Less(value) => ("<", value),
            Predicate::LessOrEqual(value) => ("<=", value),
            Predicate::In(values) => return list(f, "", values),
            Predicate::NotIn(values) => return list(f, "-", values),
            Predicate::Contains(text) => return write!(f, "~{}", Quoted(text)),
            Predicate::NotContains(text) => return write!(f, "-~{}", Quoted(text)),
            Predicate::StartsWith(text) => return write!(f, "~^{}", Quoted(text)),
            Predicate::EndsWith(text) => return write!(f, "~${}", Quoted(text)),
        };
        write!(f, "{operator}{value}")
    }
}

/// Writes `[a,b]` after `prefix`.
fn list(f: &mut fmt::Formatter<'_>, prefix: &str, values: &[Literal]) -> fmt::Result {
    write!(f, "{prefix}[")?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            f.write_char(',')?;
        }
        write!(f, "{value}")?;
    }
    f.write_char(']')
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "{}", Quoted(text)),
            Self::Integer(number) => write!(f, "{number}"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Null => f.write_str("null"),
        }
    }
}

/// Text in single quotes, with `'` and `\` escaped.
struct Quoted<'a>(&'a str);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('\'')?;
        for c in self.0.chars() {
            if matches!(c, '\'' | '\\') {
                f.write_char('\\')?;
            }
            f.write_char(c)?;
        }
        f.write_char('\'')
    }
}
