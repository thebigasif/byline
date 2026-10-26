//! A parsed `filter`.

/// A `filter` in Ghost's syntax, parsed: `tag:news`, `tag:-draft`,
/// `title:'Don\'t panic'`. Comparisons, lists, text matching and joining
/// with `+` and `,` follow in the next parts of task 1.17.
///
/// # Examples
///
/// ```
/// use khalid_api::query::{Condition, Filter, Literal, Predicate};
///
/// let draft = Predicate::IsNot(Literal::Text("draft".into()));
/// assert_eq!("tag:-draft".parse(), Ok(Filter::Condition(Condition::new("tag", draft))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Filter {
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
