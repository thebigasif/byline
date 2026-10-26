//! `khalid theme sync <url>`: read the main site's colours and fonts, fix
//! the ones too faint to read, and write `theme/tokens.toml` (task 3.8).

use std::fmt::Write as _;
use std::path::Path;

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_theme::{ContrastFix, SiteSource, SiteTheme, ThemeTokens, fetch_site};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::{outbound, report, theme_message};

/// The JSON shape of a finished sync.
#[derive(Serialize)]
struct SyncBody<'a> {
    /// The file written.
    tokens: String,
    /// The address the theme was read from.
    site: &'a str,
    /// Each colour the contrast pass changed, in file order.
    contrast_fixes: Vec<FixBody>,
}

/// The JSON shape of one contrast fix.
#[derive(Serialize)]
struct FixBody {
    token: &'static str,
    before: String,
    after: String,
    before_ratio: f64,
    after_ratio: f64,
    meets_target: bool,
    message: String,
}

impl From<&ContrastFix> for FixBody {
    fn from(fix: &ContrastFix) -> Self {
        Self {
            token: fix.token().name(),
            before: fix.before().to_hex(),
            after: fix.after().to_hex(),
            before_ratio: fix.before_ratio(),
            after_ratio: fix.after_ratio(),
            meets_target: fix.meets_target(),
            message: fix.to_string(),
        }
    }
}

/// Syncs the theme of the main site at `url` into the `site` folder.
pub(crate) fn run(site: &Path, url: &AbsoluteUrl, cli: &Cli) -> Exit {
    let output = cli.output;
    let source = match read(cli, url) {
        Ok(source) => source,
        Err((message, exit)) => return report::failure(output, &message, exit),
    };
    if source.stylesheets().is_empty() {
        let message = theme_message::no_stylesheet(url, source.skipped().len());
        return report::failure(output, &message, Exit::Failure);
    }
    let mut tokens = ThemeTokens::from_theme(&SiteTheme::from_source(&source));
    let fixes = tokens.fix_contrast();
    match tokens.write(site) {
        Ok(path) => success(output, url, &path, &fixes),
        Err(error) => report::failure(output, &theme_message::tokens(&error), Exit::Failure),
    }
}

/// Reads the home page at `url` and its stylesheets, on a runtime of its own.
fn read(cli: &Cli, url: &AbsoluteUrl) -> Result<SiteSource, (UserMessage, Exit)> {
    let internal = || (theme_message::internal(), Exit::Internal);
    let client = outbound::client(cli).map_err(|_| internal())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| internal())?;
    runtime
        .block_on(fetch_site(&client, url))
        .map_err(|error| (theme_message::fetch(url, &error), Exit::Failure))
}

/// Says where the tokens went and what the contrast pass changed.
fn success(output: Output, url: &AbsoluteUrl, path: &Path, fixes: &[ContrastFix]) -> Exit {
    match output {
        Output::Human => {
            let mut said = format!("Synced the theme from {url} into {}.\n", path.display());
            for fix in fixes {
                let _ = writeln!(said, "{fix}");
            }
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &SyncBody {
                tokens: path.display().to_string(),
                site: url.as_str(),
                contrast_fixes: fixes.iter().map(FixBody::from).collect(),
            },
            Exit::Success,
        ),
    }
}
