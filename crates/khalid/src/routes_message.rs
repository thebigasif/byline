//! What a person reads when `routes.toml` is wrong (RULES.md BL-ERR-4).

use khalid_core::UserMessage;
use khalid_routes::RoutesError;

use crate::config_message::located;

/// Every routes message shares this reason.
const WHY: &str = "Khalid checks routes.toml before it starts, so no page gets a wrong address.";

/// Turns a routes error into what happened, why it matters and what to do.
pub(crate) fn of(error: &RoutesError) -> UserMessage {
    match error {
        RoutesError::Read { path, .. } => UserMessage::new(
            format!("Khalid could not read {}.", path.display()),
            "routes.toml shapes every address on the site, so Khalid needs to read it.",
            format!(
                "Check that {} is a file you can open, or remove it to use the defaults.",
                path.display()
            ),
        ),
        RoutesError::Parse {
            path,
            line,
            field,
            source,
            ..
        } => unknown_field(&path.display().to_string(), *line, field, source.message())
            .unwrap_or_else(|| {
                located(
                    &path.display().to_string(),
                    *line,
                    field,
                    source.message(),
                    WHY,
                )
            }),
        RoutesError::Field {
            path,
            line,
            field,
            problem,
        } => located(&path.display().to_string(), *line, field, problem, WHY),
        _ => UserMessage::new(
            "Khalid could not read the site's routes.",
            WHY,
            "Check routes.toml in the folder given to --site, then run the command again.",
        ),
    }
}

/// The message for a field the file may not have. The reader names the one
/// allowed field without "one of", which the shared wording would read as a
/// value to set, so this says to remove the field instead.
fn unknown_field(path: &str, line: usize, field: &str, detail: &str) -> Option<UserMessage> {
    let (what, allowed) = detail.split_once(", expected ")?;
    if !what.starts_with("unknown field") {
        return None;
    }
    let allowed = allowed.strip_prefix("one of ").unwrap_or(allowed);
    Some(UserMessage::new(
        format!("{path}, line {line}, `{field}`: {what}."),
        WHY,
        format!("Remove `{field}` on line {line}, or use {allowed}."),
    ))
}
