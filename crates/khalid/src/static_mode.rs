//! Static mode in the binary (KH-OPS-1): the features a static site has
//! off, as the owner reads them in `khalid build` and `khalid doctor`, and
//! each built page with its forms locked (ADR "static mode locks forms
//! after the page is rendered").

use std::fmt::Write as _;

use khalid_core::{Locale, Mode, UserMessage};
use khalid_static::{LockError, Locks, ServerFeature};
use serde::Serialize;

/// The fix for most locks: the one change that brings every feature back.
const SWITCH: &str =
    "To turn this feature on, set mode = \"server\" in site.toml and run `khalid serve`.";

/// One feature that is off, with what, why and fix, as `--json` writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Off {
    pub(crate) feature: ServerFeature,
    #[serde(flatten)]
    pub(crate) message: UserMessage,
}

/// Every feature `mode` switches off, in the order Khalid lists them. Only a
/// static site has any.
pub(crate) fn off(mode: Mode) -> Vec<Off> {
    if mode != Mode::Static {
        return Vec::new();
    }
    ServerFeature::ALL
        .into_iter()
        .map(|feature| Off {
            feature,
            message: message(feature),
        })
        .collect()
}

/// What the owner reads about `feature` being off: what, why and fix.
fn message(feature: ServerFeature) -> UserMessage {
    match feature {
        ServerFeature::Comments => UserMessage::new(
            "Comments are off.",
            "A static host only sends files, so nothing can keep a reader's comment.",
            SWITCH,
        ),
        ServerFeature::Newsletter => UserMessage::new(
            "Newsletter sign-up is off.",
            "Nothing on a static host can keep a reader's email address, so each sign-up form shows a short note instead.",
            SWITCH,
        ),
        ServerFeature::Forms => UserMessage::new(
            "Forms are off.",
            "A static host cannot receive what a reader sends, so each form shows a short note instead.",
            SWITCH,
        ),
        ServerFeature::Search => UserMessage::new(
            "Search is off.",
            "Search needs a server to look through the posts while a reader types.",
            SWITCH,
        ),
        ServerFeature::Admin => UserMessage::new(
            "The admin is not part of the built site.",
            "The admin needs a running server, and a static host runs none.",
            "Edit the files in content on your own computer, then run `khalid export` again.",
        ),
        ServerFeature::ScheduledPublish => UserMessage::new(
            "Scheduled publish is off.",
            "A static site changes only when you export it, so a post set for later waits for the next export.",
            "Run `khalid export` on a schedule, or set mode = \"server\" in site.toml and run `khalid serve`.",
        ),
        _ => UserMessage::new(
            format!("The {} feature is off.", feature.name()),
            "It needs a running server, and a static host runs none.",
            SWITCH,
        ),
    }
}

/// The lines a person reads under a build or doctor report: a heading, then
/// each feature as what, why and fix. Empty when nothing is off.
pub(crate) fn human(off: &[Off]) -> String {
    let mut said = String::new();
    if off.is_empty() {
        return said;
    }
    let _ = writeln!(
        said,
        "The site is static, so {} features that need a server are off.",
        off.len()
    );
    for Off { message, .. } in off {
        let _ = writeln!(
            said,
            "{}\n  {}\n  {}",
            message.what(),
            message.why(),
            message.fix()
        );
    }
    said
}

/// The notes that stand in for forms, when `mode` is static.
///
/// # Errors
///
/// A [`UserMessage`] when the built-in notes cannot be read.
pub(crate) fn locks(mode: Mode) -> Result<Option<Locks>, UserMessage> {
    if mode != Mode::Static {
        return Ok(None);
    }
    Locks::load().map(Some).map_err(|error| {
        UserMessage::new(
            format!("Khalid could not read its notes for static pages: {error}."),
            "Each static page needs them to stand in for its forms, so nothing was built.",
            "Report it at https://github.com/thebigasif/khalid/issues with the version number.",
        )
    })
}

/// `html`, the page written to `file` in `locale`, with its forms locked
/// when there are `locks`, or as it is when there are none.
///
/// # Errors
///
/// A [`UserMessage`] when a form on the page never ends.
pub(crate) fn lock(
    locks: Option<&Locks>,
    html: String,
    locale: &Locale,
    file: &str,
) -> Result<String, UserMessage> {
    match locks {
        Some(locks) => locks
            .page(&html, locale)
            .map(|page| page.html)
            .map_err(|error| lock_message(file, &error)),
        None => Ok(html),
    }
}

/// The message when the page written to `file` cannot be locked.
fn lock_message(file: &str, error: &LockError) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not make the static page {file}: {error}."),
        "A static page must not keep a form that sends to the site, and a form left open hides where the page goes on.",
        "Close the form in the template or post that writes it, then build again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_or_headless_site_has_nothing_off() {
        assert_eq!(off(Mode::Server), []);
        assert_eq!(off(Mode::Headless), []);
        assert_eq!(human(&[]), "");
    }

    #[test]
    fn every_lock_has_a_what_a_why_and_a_fix() {
        let off = off(Mode::Static);
        assert_eq!(off.len(), ServerFeature::ALL.len());
        for Off { feature, message } in &off {
            for part in [message.what(), message.why(), message.fix()] {
                assert!(part.ends_with('.'), "{feature:?}: {part}");
                assert!(!part.contains('\u{2014}'), "{feature:?}: {part}");
            }
        }
    }
}
