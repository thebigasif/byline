//! What a person reads when the site's settings are wrong (RULES.md BL-ERR-4).

use khalid_core::{ConfigError, UserMessage};

/// Every settings message shares this reason.
const WHY: &str = "Khalid checks every setting before it starts, so nothing runs with a wrong one.";

/// Turns a settings error into what happened, why it matters and what to do.
pub(crate) fn of(error: &ConfigError) -> UserMessage {
    match error {
        ConfigError::Read { path, .. } => UserMessage::new(
            format!("Khalid could not read {}.", path.display()),
            "Every site folder needs a site.toml that holds its settings.",
            "Check the folder given to --site, or run `khalid init` to make a site there.",
        ),
        ConfigError::Parse {
            path,
            line,
            field,
            source,
            ..
        } => located(
            &path.display().to_string(),
            *line,
            field,
            source.message(),
            WHY,
        ),
        ConfigError::Field {
            path,
            line,
            field,
            problem,
        } => located(&path.display().to_string(), *line, field, problem, WHY),
        ConfigError::Environment { variable, .. } => UserMessage::new(
            format!("The environment variable {variable} is not a full web address."),
            "It replaces the address in site.toml, so a wrong value would break every link.",
            format!("Set {variable} to an address such as https://www.example.com, or unset it."),
        ),
        _ => UserMessage::new(
            "Khalid could not read the site's settings.",
            WHY,
            "Check site.toml in the folder given to --site, then run the command again.",
        ),
    }
}

/// A message for a mistake on one line of a settings or content file. `detail`
/// keeps its first clause, and what is allowed instead, such as `server` or
/// `static`, or the fields a file may have, moves to the fix.
pub(crate) fn located(
    path: &str,
    line: usize,
    field: &str,
    detail: &str,
    why: &'static str,
) -> UserMessage {
    let (detail, allowed) = match detail.split_once(", expected ") {
        Some((detail, allowed)) => (detail, Some(allowed)),
        None => (detail, None),
    };
    let detail = detail.trim_end_matches('.');
    if field.is_empty() {
        return UserMessage::new(
            format!("{path}, line {line}: {detail}."),
            why,
            format!("Correct line {line}, then run the command again."),
        );
    }
    let fix = match allowed {
        Some(fields) if detail.starts_with("unknown field") => unknown_field(field, line, fields),
        Some(allowed) => {
            format!("Set `{field}` on line {line} to {allowed}, then run the command again.")
        }
        None => format!("Correct `{field}` on line {line}, then run the command again."),
    };
    UserMessage::new(
        format!("{path}, line {line}, `{field}`: {detail}."),
        why,
        fix,
    )
}

/// The most fields a fix lists; past it the sentence grows too long to read.
const MOST_LISTED: usize = 12;

/// The fix for `field`, which no file of its kind has, given the `fields`
/// one may have, such as "one of title, slug, date". A long list is not
/// read out: the fix names the field one or two letters away, if any.
fn unknown_field(field: &str, line: usize, fields: &str) -> String {
    let names: Vec<&str> = fields
        .strip_prefix("one of ")
        .unwrap_or(fields)
        .split(", ")
        .map(|name| name.trim_matches('`'))
        .collect();
    if names.len() <= MOST_LISTED {
        return format!("Remove `{field}` on line {line}, or use {fields}.");
    }
    let near = names.iter().min_by_key(|name| distance(field, name));
    match near.filter(|name| distance(field, name) <= 2) {
        Some(name) => {
            format!("Remove `{field}` on line {line}, or write `{name}` if you meant it.")
        }
        None => {
            format!("Remove `{field}` on line {line}, because Khalid reads no field by that name.")
        }
    }
}

/// How many letters must change, be added or be removed to turn `a` into
/// `b` (the Levenshtein distance).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut diagonal = row.first().copied().unwrap_or(i);
        let mut previous = i + 1;
        let mut next = vec![previous];
        for (j, right) in b.iter().enumerate() {
            let above = row.get(j + 1).copied().unwrap_or(0);
            let cost = diagonal + usize::from(left != *right);
            previous = cost.min(above + 1).min(previous + 1);
            next.push(previous);
            diagonal = above;
        }
        row = next;
    }
    row.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POST_FIELDS: &str = "one of title, slug, date, author, category, description, \
        translation, aliases, featured, robots, canonical, reviewed_by, related";

    #[test]
    fn a_typo_in_a_long_list_names_the_field_it_may_mean() {
        let fix = unknown_field("titles", 2, POST_FIELDS);
        assert_eq!(
            fix,
            "Remove `titles` on line 2, or write `title` if you meant it."
        );
        let fix = unknown_field("colour", 5, POST_FIELDS);
        assert_eq!(
            fix,
            "Remove `colour` on line 5, because Khalid reads no field by that name."
        );
        let short = unknown_field("mod", 1, "one of `server`, `static`");
        assert_eq!(
            short,
            "Remove `mod` on line 1, or use one of `server`, `static`."
        );
    }

    #[test]
    fn distance_counts_the_letters_to_change() {
        assert_eq!(distance("titles", "title"), 1);
        assert_eq!(distance("", "date"), 4);
        assert_eq!(distance("slug", "slug"), 0);
        assert_eq!(distance("authors", "author"), 1);
    }
}
