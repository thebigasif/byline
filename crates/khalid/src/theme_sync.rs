//! `khalid theme sync <url>`: read the main site's colours and fonts, fix
//! the ones too faint to read, and write `theme/tokens.toml`, keeping every
//! token the file locks (task 3.8).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_theme::{
    ContrastFix, SiteSource, SiteTheme, ThemeDiff, ThemeTokens, TokenChange, fetch_site,
};
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
    /// Each token that differed from the file already there, in file
    /// order. Absent on a first sync.
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<Vec<ChangeBody>>,
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

/// The JSON shape of one token whose file value and site value differed.
/// `file` is the value the file had, `site` the main site's; `kept` says
/// whether the file kept its value because the token is locked.
#[derive(Serialize)]
pub(crate) struct ChangeBody {
    token: &'static str,
    kept: bool,
    file: String,
    site: String,
    message: String,
}

impl ChangeBody {
    /// Every change in `diff`, in file order.
    pub(crate) fn all(diff: &ThemeDiff) -> Vec<Self> {
        diff.changes().iter().filter_map(Self::of).collect()
    }

    fn of(change: &TokenChange) -> Option<Self> {
        let (name, kept, file, site) = match change {
            TokenChange::Updated {
                name,
                before,
                after,
            } => (name, false, before, after),
            TokenChange::Kept { name, kept, site } => (name, true, kept, site),
            _ => return None,
        };
        Some(Self {
            token: name.as_str(),
            kept,
            file: file.clone(),
            site: site.clone(),
            message: change.to_string(),
        })
    }
}

/// What one sync wrote.
struct Synced {
    path: PathBuf,
    fixes: Vec<ContrastFix>,
    diff: Option<ThemeDiff>,
}

/// Syncs the theme of the main site at `url` into the `site` folder.
pub(crate) fn run(site: &Path, url: &AbsoluteUrl, cli: &Cli) -> Exit {
    let output = cli.output;
    let (fresh, mut fixes) = match site_tokens(cli, url) {
        Ok(found) => found,
        Err((message, exit)) => return report::failure(output, &message, exit),
    };
    let (tokens, diff) = match ThemeTokens::read(site) {
        Ok(Some(mut file)) => {
            fixes.retain(|fix| !file.locked.iter().any(|n| n.as_str() == fix.token().name()));
            let diff = file.resync(&fresh);
            (file, Some(diff))
        }
        Ok(None) => (fresh, None),
        Err(error) => return report::failure(output, &theme_message::read(&error), Exit::Failure),
    };
    match tokens.write(site) {
        Ok(path) => success(output, url, &Synced { path, fixes, diff }),
        Err(error) => report::failure(output, &theme_message::tokens(&error), Exit::Failure),
    }
}

/// The tokens the main site at `url` gives today, after the contrast pass,
/// and what that pass changed.
pub(crate) fn site_tokens(
    cli: &Cli,
    url: &AbsoluteUrl,
) -> Result<(ThemeTokens, Vec<ContrastFix>), (UserMessage, Exit)> {
    let source = read(cli, url)?;
    if source.stylesheets().is_empty() {
        let message = theme_message::no_stylesheet(url, source.skipped().len());
        return Err((message, Exit::Failure));
    }
    let mut tokens = ThemeTokens::from_theme(&SiteTheme::from_source(&source));
    let fixes = tokens.fix_contrast();
    Ok((tokens, fixes))
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

/// Says where the tokens went, what the contrast pass changed and, on a
/// sync after the first, what changed in the file.
fn success(output: Output, url: &AbsoluteUrl, synced: &Synced) -> Exit {
    let path = synced.path.display();
    match output {
        Output::Human => {
            let mut said = format!("Synced the theme from {url} into {path}.\n");
            for fix in &synced.fixes {
                let _ = writeln!(said, "{fix}");
            }
            if let Some(diff) = &synced.diff {
                let _ = write!(said, "{diff}");
            }
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &SyncBody {
                tokens: path.to_string(),
                site: url.as_str(),
                contrast_fixes: synced.fixes.iter().map(FixBody::from).collect(),
                changes: synced.diff.as_ref().map(ChangeBody::all),
            },
            Exit::Success,
        ),
    }
}
