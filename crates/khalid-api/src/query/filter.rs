//! A parsed `filter`: conditions joined with and and or.

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
