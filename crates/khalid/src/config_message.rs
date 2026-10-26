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
        Some(fields) if fields.starts_with("one of ") => {
            format!("Remove `{field}` on line {line}, or use {fields}.")
        }
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
