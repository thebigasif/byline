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
        Some(fields) if detail.starts_with("unknown field") => unknown_fix(field, line, fields),
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

/// The most fields a fix lists by name before the sentence grows too long
/// to read in one go (BL-CLI-3).
const MAX_LISTED: usize = 12;

/// What to do about the unknown `field`, given the fields `allowed` there,
/// such as `one of title, slug`: rename a near miss, list a short set, or
/// remove it.
fn unknown_fix(field: &str, line: usize, allowed: &str) -> String {
    let names: Vec<&str> = allowed
        .trim_start_matches("one of ")
        .split(", ")
        .flat_map(|part| part.split(" or "))
        .map(|name| name.trim_matches('`'))
        .collect();
    let nearest = names
        .iter()
        .map(|name| (distance(field, name), *name))
        .filter(|(edits, _)| *edits <= 2)
        .min();
    match nearest {
        Some((_, name)) => format!("Remove `{field}` on line {line}, or rename it `{name}`."),
        None if names.len() <= MAX_LISTED => {
            format!("Remove `{field}` on line {line}, or use {allowed}.")
        }
        None => format!("Remove `{field}` on line {line}, then run the command again."),
    }
}

/// How many single-letter edits turn `a` into `b` (Levenshtein distance).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut diagonal = i;
        let mut next = vec![i.saturating_add(1)];
        for (j, right) in b.iter().enumerate() {
            let above = row.get(j.saturating_add(1)).copied().unwrap_or(usize::MAX);
            let before = next.last().copied().unwrap_or(usize::MAX);
            let swap = diagonal.saturating_add(usize::from(left != *right));
            next.push(
                swap.min(above.saturating_add(1))
                    .min(before.saturating_add(1)),
            );
            diagonal = above;
        }
        row = next;
    }
    row.last().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_near_miss_is_renamed_and_a_long_list_is_not_printed() {
        let allowed = "one of title, slug, date, author, category, description, translation, aliases, featured, robots, canonical, reviewed_by, related";
        assert_eq!(
            unknown_fix("slgu", 3, allowed),
            "Remove `slgu` on line 3, or rename it `slug`."
        );
        assert_eq!(
            unknown_fix("colour", 3, allowed),
            "Remove `colour` on line 3, then run the command again."
        );
        assert_eq!(
            unknown_fix("colour", 3, "`server` or `static`"),
            "Remove `colour` on line 3, or use `server` or `static`."
        );
        assert_eq!(distance("kitten", "sitting"), 3);
    }

    #[test]
    fn a_typo_in_a_long_list_names_the_field_it_may_mean() {
        let allowed = "one of title, slug, date, author, category, description, \
            translation, aliases, featured, robots, canonical, reviewed_by, related";
        assert_eq!(
            unknown_fix("titles", 2, allowed),
            "Remove `titles` on line 2, or rename it `title`."
        );
        assert_eq!(
            unknown_fix("mod", 1, "one of `server`, `static`"),
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
