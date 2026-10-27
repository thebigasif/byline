//! `khalid theme sync <url>`: read the main site's colours and fonts, fix
//! the ones too faint to read, write `theme/tokens.toml`, keeping every
//! token the file locks, and draw a preview page (task 3.8, plan 4.9).

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use khalid_core::{AbsoluteUrl, Locale, UserMessage};
use khalid_render::Renderer;
use khalid_theme::{
    ContrastFix, FontFileEntry, FontSource, HeldFont, MAX_CSS_BYTES, SiteSource, SiteTheme,
    ThemeDiff, ThemeTokens, TokenChange, download_and_report, fetch_site,
};
use serde::Serialize;

use crate::cli::{Cli, Output};
use crate::exit::Exit;
use crate::theme_preview::{Preview, PreviewError, Source};
use crate::{outbound, report, theme_message};

/// The JSON shape of a finished sync.
#[derive(Serialize)]
struct SyncBody<'a> {
    /// The file written.
    tokens: String,
    /// The preview page written, ready to open in a browser.
    preview: String,
    /// The address the theme was read from.
    site: &'a str,
    /// The CSS file the colours and fonts came from, when `--from-css`
    /// was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    from_css: Option<String>,
    /// Each colour the contrast pass changed, in file order.
    contrast_fixes: Vec<FixBody>,
    /// Each token that differed from the file already there, in file
    /// order. Absent on a first sync.
    #[serde(skip_serializing_if = "Option::is_none")]
    changes: Option<Vec<ChangeBody>>,
    /// Whether the home page is drawn by JavaScript, so the clean default
    /// look was used and `--from-css` can do better.
    drawn_by_script: bool,
    /// Each font the blog may not serve, in page order.
    held_fonts: Vec<HeldBody<'a>>,
    /// Why a font that may be served could not be, in plain words.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    skipped_fonts: Vec<String>,
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
struct Synced<'a> {
    path: PathBuf,
    preview: PathBuf,
    from_css: Option<&'a Path>,
    fixes: Vec<ContrastFix>,
    diff: Option<ThemeDiff>,
    notes: Notes,
}

/// What the main site gives today.
pub(crate) struct FromSite {
    /// The tokens, after the contrast pass.
    pub(crate) tokens: ThemeTokens,
    /// The font files the site loads, for the download that follows.
    pub(crate) sources: Vec<FontSource>,
    /// What the contrast pass changed.
    pub(crate) fixes: Vec<ContrastFix>,
    /// The site's name, when its home page says it.
    pub(crate) name: Option<String>,
    /// What the person should know about what could not be copied.
    pub(crate) notes: Notes,
}

/// What a sync could not copy, said in plain words beside the changes.
pub(crate) struct Notes {
    /// The home page is drawn by JavaScript, so the default look was used.
    drawn_by_script: bool,
    /// The fonts the blog may not serve.
    held: Vec<HeldFont>,
    /// Why a font that may be served could not be, in plain words.
    pub(crate) skipped_fonts: Vec<String>,
}

impl FromSite {
    /// The tokens for `theme`, after the contrast pass.
    fn of(theme: SiteTheme) -> Self {
        let mut tokens = ThemeTokens::from_theme(&theme);
        let fixes = tokens.fix_contrast();
        let sources = theme.typography.sources.clone();
        Self {
            tokens,
            fixes,
            sources,
            name: theme.brand.name,
            notes: Notes {
                drawn_by_script: theme.drawn_by_script,
                held: theme.typography.held,
                skipped_fonts: Vec::new(),
            },
        }
    }
}

/// The JSON shape of one font the blog may not serve.
#[derive(Serialize)]
struct HeldBody<'a> {
    family: &'a str,
    server: &'a str,
    message: String,
}

/// Syncs the theme of the main site at `url` into the `site` folder, and
/// draws the preview in `locale`, the blog's own language. With `from_css`,
/// the colours and fonts come from that file and nothing is fetched.
pub(crate) fn run(
    site: &Path,
    url: &AbsoluteUrl,
    from_css: Option<&Path>,
    locale: &Locale,
    cli: &Cli,
) -> Exit {
    let output = cli.output;
    let found = match from_css {
        Some(file) => css_tokens(file),
        None => site_tokens(site, cli, url),
    };
    let FromSite {
        tokens: fresh,
        mut fixes,
        sources: _,
        name,
        notes,
    } = match found {
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
    let path = match tokens.write(site) {
        Ok(path) => path,
        Err(error) => {
            return report::failure(output, &theme_message::tokens(&error), Exit::Failure);
        }
    };
    let source = Source {
        name: name.as_deref(),
        url,
        locale,
    };
    match preview(site, &tokens, source) {
        Ok(preview) => success(
            output,
            url,
            &Synced {
                path,
                preview,
                from_css,
                fixes,
                diff,
                notes,
            },
        ),
        Err(error) => report::failure(output, &error.message(), Exit::Failure),
    }
}

/// Draws the sample post with `tokens` through the site's own templates and
/// writes it under `data/theme-preview/`.
fn preview(site: &Path, tokens: &ThemeTokens, source: Source<'_>) -> Result<PathBuf, PreviewError> {
    let renderer = Renderer::with_theme(site).map_err(PreviewError::Render)?;
    Preview::render(&renderer, tokens, source)?.write(site)
}

/// The tokens the main site at `url` gives today, after the contrast pass,
/// what that pass changed, and the site's name. A page drawn by JavaScript
/// with no stylesheet gives the clean default look (plan section 4.9).
pub(crate) fn site_tokens(
    site: &Path,
    cli: &Cli,
    url: &AbsoluteUrl,
) -> Result<FromSite, (UserMessage, Exit)> {
    let source = read(cli, url)?;
    let theme = SiteTheme::from_source(&source);
    if source.stylesheets().is_empty() && !theme.drawn_by_script {
        let message = theme_message::no_stylesheet(url, source.skipped().len());
        return Err((message, Exit::Failure));
    }
    let mut found = FromSite::of(theme);
    // Fonts the licence lets the blog serve itself are downloaded beside the
    // tokens, so no reader waits on a third-party server (G20). A font that
    // cannot be read is skipped; the stack's next font stands in.
    if !found.sources.is_empty() {
        let client =
            outbound::client(cli).map_err(|_| (theme_message::internal(), Exit::Internal))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| (theme_message::internal(), Exit::Internal))?;
        let mut folder = site.to_path_buf();
        folder.push("theme");
        folder.push("fonts");
        let (saved, skipped) = runtime
            .block_on(download_and_report(&client, &found.sources, &folder))
            .map_err(|_| (theme_message::internal(), Exit::Internal))?;
        found.notes.skipped_fonts = skipped;
        found.tokens.fonts.files = saved
            .into_iter()
            .map(|font| FontFileEntry {
                family: font.family,
                style: font.style,
                weight: font.weight,
                path: font.path,
                range: font.range,
            })
            .collect();
    }
    Ok(found)
}

/// The tokens the CSS in `file` gives, after the contrast pass. The file
/// is read up to the cap one fetched stylesheet has.
fn css_tokens(file: &Path) -> Result<FromSite, (UserMessage, Exit)> {
    let read = || -> io::Result<String> {
        let mut css = String::new();
        let cap = u64::try_from(MAX_CSS_BYTES).unwrap_or(u64::MAX);
        fs::File::open(file)?
            .take(cap.saturating_add(1))
            .read_to_string(&mut css)?;
        if css.len() > MAX_CSS_BYTES {
            return Err(io::ErrorKind::FileTooLarge.into());
        }
        Ok(css)
    };
    let css = read().map_err(|error| (theme_message::css_file(file, &error), Exit::Failure))?;
    Ok(FromSite::of(SiteTheme::from_css(&css)))
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
fn success(output: Output, url: &AbsoluteUrl, synced: &Synced<'_>) -> Exit {
    let path = synced.path.display();
    let preview = synced.preview.display();
    match output {
        Output::Human => {
            let from = synced.from_css.map_or_else(
                || url.to_string(),
                |file| format!("the CSS in {}", file.display()),
            );
            let mut said = format!("Synced the theme from {from} into {path}.\n");
            for fix in &synced.fixes {
                let _ = writeln!(said, "{fix}");
            }
            if let Some(diff) = &synced.diff {
                let _ = write!(said, "{diff}");
            }
            if synced.notes.drawn_by_script {
                let _ = writeln!(said, "{}", theme_message::drawn_by_script(url));
            }
            for font in &synced.notes.held {
                let _ = writeln!(said, "{}", theme_message::held_font(font));
            }
            for why in &synced.notes.skipped_fonts {
                let _ = writeln!(said, "{why}");
            }
            let _ = writeln!(
                said,
                "Open {preview} in a browser to see how the blog will look."
            );
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &SyncBody {
                tokens: path.to_string(),
                preview: preview.to_string(),
                site: url.as_str(),
                from_css: synced.from_css.map(|file| file.display().to_string()),
                contrast_fixes: synced.fixes.iter().map(FixBody::from).collect(),
                changes: synced.diff.as_ref().map(ChangeBody::all),
                drawn_by_script: synced.notes.drawn_by_script,
                held_fonts: synced
                    .notes
                    .held
                    .iter()
                    .map(|font| HeldBody {
                        family: font.family(),
                        server: font.server(),
                        message: theme_message::held_font(font),
                    })
                    .collect(),
                skipped_fonts: synced.notes.skipped_fonts.clone(),
            },
            Exit::Success,
        ),
    }
}
