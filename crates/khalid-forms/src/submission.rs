//! A form's answers, read once from what a browser posted.

use crate::FormId;

/// The spam trap's field, which the form block hides from people, so only
/// robots fill it in.
pub(crate) const TRAP: &str = "homepage";

/// The most fields one form may send, well above what a reader fills in.
const MOST_FIELDS: usize = 30;

/// The longest field name, in bytes.
const LONGEST_NAME: usize = 64;

/// The longest answer to one field, in characters.
pub(crate) const LONGEST_ANSWER: usize = 5_000;

/// One kept answer to a form: which form, the page it is on, and each
/// field's name and value in the order the form sent them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    /// The form it answers.
    pub form: FormId,
    /// The address of the page the form is on, such as `/blog/contact/`.
    pub page: String,
    /// Each field's name, every one different, and its value, without the
    /// spam trap.
    pub fields: Vec<(String, String)>,
}

/// What a posted form turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reading {
    /// Real answers, to keep.
    Answers(Vec<(String, String)>),
    /// A robot filled in the trap.
    Bait,
    /// Nothing to keep, and why.
    Problem(Problem),
}

/// Why a posted form is not kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Problem {
    /// Every field was empty.
    Empty,
    /// One answer is longer than [`LONGEST_ANSWER`].
    TooLong,
    /// The body was not a form, or had too many fields, a field name the
    /// form block never writes, or one name twice.
    Unreadable,
}

/// Reads the `pairs` a browser posted.
pub(crate) fn read(pairs: Vec<(String, String)>) -> Reading {
    if pairs
        .iter()
        .any(|(name, value)| name == TRAP && !value.is_empty())
    {
        return Reading::Bait;
    }
    let fields: Vec<(String, String)> =
        pairs.into_iter().filter(|(name, _)| name != TRAP).collect();
    let plain = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_';
    let readable =
        |name: &str| !name.is_empty() && name.len() <= LONGEST_NAME && name.bytes().all(plain);
    let mut sorted: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    sorted.sort_unstable();
    let repeated = sorted.windows(2).any(|pair| pair.first() == pair.get(1));
    if fields.len() > MOST_FIELDS || repeated || !sorted.iter().all(|name| readable(name)) {
        return Reading::Problem(Problem::Unreadable);
    }
    if fields
        .iter()
        .any(|(_, value)| value.chars().count() > LONGEST_ANSWER)
    {
        return Reading::Problem(Problem::TooLong);
    }
    if fields.iter().all(|(_, value)| value.trim().is_empty()) {
        return Reading::Problem(Problem::Empty);
    }
    Reading::Answers(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|&(n, v)| (n.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn the_trap_is_dropped_when_empty_and_bait_when_filled() {
        let read_from = |list: &[(&str, &str)]| read(pairs(list));
        assert_eq!(
            read_from(&[("name", "Ada"), (TRAP, "")]),
            Reading::Answers(pairs(&[("name", "Ada")]))
        );
        assert_eq!(read_from(&[("name", "Ada"), (TRAP, "x")]), Reading::Bait);
        assert_eq!(read_from(&[]), Reading::Problem(Problem::Empty));
        assert_eq!(
            read_from(&[("name", "Ada"), ("name", "Bo")]),
            Reading::Problem(Problem::Unreadable)
        );
        let many: Vec<(String, String)> =
            (0..31).map(|i| (format!("f{i}"), "a".to_owned())).collect();
        assert_eq!(read(many), Reading::Problem(Problem::Unreadable));
    }
}
