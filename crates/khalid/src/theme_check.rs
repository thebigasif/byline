//! `khalid theme check [url]`: compare `theme/tokens.toml` with the main
//! site and with WCAG contrast, and exit with 1 when either needs a person
//! (task 3.8). It writes nothing.

use std::fmt::Write as _;
use std::path::Path;

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_theme::{ContrastFix, ThemeTokens, TokenChange};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::theme_sync::{ChangeBody, site_tokens};
use crate::{report, theme_message};

/// The JSON shape of a check.
#[derive(Serialize)]
struct CheckBody<'a> {
    /// The address the file was compared with.
    site: &'a str,
    /// Whether the file matches the site, apart from locked tokens, and
    /// every colour is readable.
    ok: bool,
    /// Each token whose file value differs from the site's, in file order.
    /// A locked one has `kept` set and is not a problem.
    drift: Vec<ChangeBody>,
    /// Each colour too faint on the background or the surface.
    contrast: Vec<FaintBody>,
}

/// The JSON shape of one colour that fails contrast.
#[derive(Serialize)]
struct FaintBody {
    token: &'static str,
    color: String,
    ratio: f64,
    target: f64,
    /// The nearest shade that passes, or the one with the most contrast
    /// when none does.
    nearest: String,
    nearest_passes: bool,
    message: String,
}

impl From<&ContrastFix> for FaintBody {
    fn from(fix: &ContrastFix) -> Self {
        Self {
            token: fix.token().name(),
            color: fix.before().to_hex(),
            ratio: fix.before_ratio(),
            target: fix.token().target(),
            nearest: fix.after().to_hex(),
            nearest_passes: fix.meets_target(),
            message: faint_sentence(fix),
        }
    }
}

/// Checks the tokens in the `site` folder against the main site at `url`.
pub(crate) fn run(site: &Path, url: &AbsoluteUrl, cli: &Cli) -> Exit {
    let output = cli.output;
    let mut file = match ThemeTokens::read(site) {
        Ok(Some(file)) => file,
        Ok(None) => return report::failure(output, &no_tokens(url), Exit::Failure),
        Err(error) => return report::failure(output, &theme_message::read(&error), Exit::Failure),
    };
    let faint = file.clone().fix_contrast();
    let (fresh, _) = match site_tokens(cli, url) {
        Ok(found) => found,
        Err((message, exit)) => return report::failure(output, &message, exit),
    };
    // The merge runs on this copy only, to see what a sync would change.
    let diff = file.resync(&fresh);
    let drifted = diff
        .changes()
        .iter()
        .filter(|change| matches!(change, TokenChange::Updated { .. }))
        .count();
    let ok = drifted == 0 && faint.is_empty();
    let exit = if ok { Exit::Success } else { Exit::Failure };
    match output {
        Output::Json => report::json(
            &CheckBody {
                site: url.as_str(),
                ok,
                drift: ChangeBody::all(&diff),
                contrast: faint.iter().map(FaintBody::from).collect(),
            },
            exit,
        ),
        Output::Human => {
            let mut said = format!("Checked theme/tokens.toml against {url}.\n");
            for change in diff.changes() {
                let _ = writeln!(said, "{}", drift(change));
            }
            for fix in &faint {
                let _ = writeln!(said, "{}", faint_sentence(fix));
            }
            said.push_str(&summary(url, drifted, faint.len()));
            report::text(&said, exit)
        }
    }
}

/// One drifted token in the present tense: the file's value and the site's.
fn drift(change: &TokenChange) -> String {
    match change {
        TokenChange::Updated {
            name,
            before,
            after,
        } => format!("{name} is {before} here, and the main site now uses {after}."),
        _ => change.to_string(),
    }
}

/// One faint colour: its contrast, the standard, and the nearest shade.
fn faint_sentence(fix: &ContrastFix) -> String {
    let token = fix.token();
    let mut said = format!(
        "{} is {}, with a contrast of {} to 1 on the page, and WCAG AA, the web's standard \
         for readable pages, asks for {} to 1.",
        token.name(),
        fix.before().to_hex(),
        ratio(fix.before_ratio()),
        ratio(token.target()),
    );
    if fix.meets_target() {
        let _ = write!(
            said,
            " The closest shade that passes is {}.",
            fix.after().to_hex()
        );
    } else {
        said.push_str(" No shade of it passes on both colors.background and colors.surface.");
    }
    said
}

/// The last line: all clear, or how many problems and what to do.
fn summary(url: &AbsoluteUrl, drifted: usize, faint: usize) -> String {
    match (drifted, faint) {
        (0, 0) => "The theme matches the main site, and every colour is readable.\n".to_owned(),
        (0, _) => "Change the faint colours in theme/tokens.toml, or unlock them and run \
                   `khalid theme sync` to take the closest readable shades.\n"
            .to_owned(),
        _ => format!(
            "Run `khalid theme sync {url}` to take the main site's values, or add a token \
             to `locked` to keep yours on purpose.\n"
        ),
    }
}

/// A contrast ratio to one decimal, rounded down, so a failing 4.47 never
/// reads as a passing 4.5.
fn ratio(value: f64) -> String {
    format!("{:.1}", (value * 10.0).floor() / 10.0)
}

/// The message when the site has no tokens file to check.
fn no_tokens(url: &AbsoluteUrl) -> UserMessage {
    UserMessage::new(
        "This site has no theme/tokens.toml yet.",
        "Theme check compares that file with the main site, so there is nothing to check.",
        format!("Run `khalid theme sync {url}` to write it, then check again."),
    )
}
