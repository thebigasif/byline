//! Applying a filter to one resource.

use std::cmp::Ordering;

use super::filter::{Filter, Literal, Predicate};

/// A resource as a filter sees it: the values of each field.
///
/// A field may hold several values, such as a post's tags, or none, such as
/// a post with no feature image, which then matches `null`. The in-memory
/// store filters with this; a database store translates the same [`Filter`]
/// into a query instead.
pub trait FieldValues {
    /// Every value of `field`. Unknown fields hold no values; check fields
    /// first with [`Filter::check_fields`].
    fn values(&self, field: &str) -> Vec<Literal>;
}

impl Filter {
    /// Whether `item` passes. A condition holds when any value of its field
    /// passes, and a negated one when none does. Text comparisons ignore
    /// case for `~`, `~^` and `~$`; values of different kinds never match.
    pub fn matches(&self, item: &impl FieldValues) -> bool {
        match self {
            Self::And(parts) => parts.iter().all(|part| part.matches(item)),
            Self::Or(parts) => parts.iter().any(|part| part.matches(item)),
            Self::Condition(condition) => {
                holds(condition.predicate(), &item.values(condition.field()))
            }
        }
    }

    /// Checks that every field the filter uses is in `allowed`.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownFieldError`] naming the first field that is not.
    pub fn check_fields(&self, allowed: &[&str]) -> Result<(), UnknownFieldError> {
        match self {
            Self::And(parts) | Self::Or(parts) => {
                parts.iter().try_for_each(|part| part.check_fields(allowed))
            }
            Self::Condition(condition) if allowed.contains(&condition.field()) => Ok(()),
            Self::Condition(condition) => Err(UnknownFieldError {
                field: condition.field().to_owned(),
            }),
        }
    }
}

fn holds(predicate: &Predicate, values: &[Literal]) -> bool {
    let any = |test: &dyn Fn(&Literal) -> bool| values.iter().any(test);
    let ordered = |value: &Literal, want: fn(Ordering) -> bool| {
        any(&|have| compare(have, value).is_some_and(want))
    };
    let text = |needle: &str, test: fn(&str, &str) -> bool| {
        let needle = needle.to_lowercase();
        any(&|have| matches!(have, Literal::Text(have) if test(&have.to_lowercase(), &needle)))
    };
    match predicate {
        Predicate::Is(value) => is(values, value),
        Predicate::IsNot(value) => !is(values, value),
        Predicate::Greater(value) => ordered(value, Ordering::is_gt),
        Predicate::GreaterOrEqual(value) => ordered(value, Ordering::is_ge),
        Predicate::Less(value) => ordered(value, Ordering::is_lt),
        Predicate::LessOrEqual(value) => ordered(value, Ordering::is_le),
        Predicate::In(list) => list.iter().any(|value| is(values, value)),
        Predicate::NotIn(list) => !list.iter().any(|value| is(values, value)),
        Predicate::Contains(needle) => text(needle, |have, want| have.contains(want)),
        Predicate::NotContains(needle) => !text(needle, |have, want| have.contains(want)),
        Predicate::StartsWith(needle) => text(needle, |have, want| have.starts_with(want)),
        Predicate::EndsWith(needle) => text(needle, |have, want| have.ends_with(want)),
    }
}

/// Equality, where `null` also matches a field with no values.
fn is(values: &[Literal], value: &Literal) -> bool {
    (values.is_empty() && *value == Literal::Null) || values.contains(value)
}

/// Orders two values of the same kind. Text orders by bytes, which is date
/// order for the ISO 8601 times the API uses.
fn compare(have: &Literal, want: &Literal) -> Option<Ordering> {
    match (have, want) {
        (Literal::Text(have), Literal::Text(want)) => Some(have.cmp(want)),
        (Literal::Integer(have), Literal::Integer(want)) => Some(have.cmp(want)),
        _ => None,
    }
}

/// A filter names a field the resource does not have.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the filter uses {field}, which is not a field here")]
pub struct UnknownFieldError {
    field: String,
}

impl UnknownFieldError {
    /// The field as written.
    pub fn field(&self) -> &str {
        &self.field
    }
}
