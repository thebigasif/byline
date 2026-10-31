//! Settings, read once when Khalid starts.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::mail_settings::MailTable;
use crate::search_console_settings::SearchConsoleTable;
use crate::toml_location::{field_of, line_of, line_of_key, line_of_key_in};
use crate::{
    AbsoluteUrl, AlertSettings, Announcement, Environment, Locale, MailSettings, MediaSettings,
    NavSettings, ParseUrlError, Reader, SearchConsoleSettings, SeoSettings, SiteCode,
};

/// Every setting Khalid reads, checked once at start and then passed by
/// reference (RULES.md BL-RUN-2).
///
/// It comes from `site.toml` in the site folder, then the environment, which
/// wins. No other module reads either source itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    name: String,
    address: AbsoluteUrl,
    mount: String,
    mode: Mode,
    slugs: SlugMode,
    default_language: Locale,
    languages: Vec<Locale>,
    seo: SeoSettings,
    reader: Reader,
    nav: NavSettings,
    announcement: Option<Announcement>,
    media: MediaSettings,
    mail: Option<MailSettings>,
    alerts: AlertSettings,
    code: SiteCode,
    search_console: Option<SearchConsoleSettings>,
    preview_link_lifetime: Duration,
}

/// Whether Khalid runs a server, writes plain HTML, or leaves the pages to
/// another app (ADR "headless is a mode").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Khalid serves the site and every server feature works.
    #[default]
    Server,
    /// Khalid writes plain HTML to `public/`, and server features are locked.
    Static,
    /// Another app renders the pages from the Content API. Khalid serves the
    /// API, feeds, sitemaps and redirects, and makes no HTML page.
    Headless,
}

impl Mode {
    /// Whether Khalid renders the site's HTML pages with its theme.
    pub fn has_pages(self) -> bool {
        match self {
            Self::Server | Self::Static => true,
            Self::Headless => false,
        }
    }
}

/// How slugs are written (`docs/decisions/2026-10-26-cto-answers.md`, section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlugMode {
    /// Lowercase a to z, digits and hyphens, transliterated by language.
    #[default]
    Ascii,
    /// Letters from any script, for languages where ASCII is unreadable.
    Unicode,
}

/// `site.toml` as written, before the checks that span several fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteFile {
    name: String,
    address: AbsoluteUrl,
    #[serde(default = "root_mount")]
    mount: String,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    slugs: SlugMode,
    default_language: Locale,
    languages: Vec<Locale>,
    #[serde(default)]
    seo: SeoSettings,
    #[serde(default)]
    reader: Reader,
    #[serde(default)]
    nav: NavSettings,
    announcement: Option<Announcement>,
    #[serde(default)]
    media: MediaSettings,
    mail: Option<MailTable>,
    #[serde(default)]
    alerts: AlertSettings,
    #[serde(default)]
    code: SiteCode,
    search_console: Option<SearchConsoleTable>,
    preview_link_days: Option<u64>,
    preview_link_hours: Option<u64>,
}

/// The value of the process's environment variable `name`, or `None` when
/// it is unset or not Unicode. Only [`crate::Environment::from_process`]
/// calls it; it lives here because this file is where settings are read
/// once at start (BL-RUN-2).
pub(crate) fn process_variable(name: &str) -> Option<String> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the one place that reads the environment (BL-RUN-2)"
    )]
    let value = std::env::var(name);
    value.ok()
}

fn root_mount() -> String {
    "/".to_owned()
}

/// An hour, the shortest a preview link may work and its unit.
const HOUR: u64 = 3600;

/// The longest a preview link may work, 30 days, in hours.
const MOST_HOURS: u64 = 30 * 24;

/// How long a preview link works when `site.toml` does not say, 7 days.
const DEFAULT_HOURS: u64 = 7 * 24;

/// The preview link lifetime `file` sets, or the key it is set under and
/// what is wrong with it.
fn preview_lifetime(file: &SiteFile) -> Result<Duration, (&'static str, &'static str)> {
    const RANGE: &str =
        "a preview link works for 1 hour to 30 days, so use 1 to 30 days or 1 to 720 hours";
    let (key, hours) = match (file.preview_link_days, file.preview_link_hours) {
        (Some(_), Some(_)) => {
            let both = "set preview_link_days or preview_link_hours, not both";
            return Err(("preview_link_hours", both));
        }
        (Some(days), None) => ("preview_link_days", days.saturating_mul(24)),
        (None, Some(hours)) => ("preview_link_hours", hours),
        (None, None) => ("", DEFAULT_HOURS),
    };
    if !(1..=MOST_HOURS).contains(&hours) {
        return Err((key, RANGE));
    }
    Ok(Duration::from_secs(hours.saturating_mul(HOUR)))
}

/// The first mistake in where `file` mounts the site and which languages it
/// names, as the key it is under and what is wrong, or `None`.
fn layout_mistake(file: &SiteFile) -> Option<(&'static str, &'static str)> {
    let mount = &file.mount;
    if !mount.starts_with('/') || !mount.ends_with('/') || mount.contains("//") {
        return Some((
            "mount",
            "the mount path must start and end with /, like /blog/",
        ));
    }
    if file.languages.is_empty() {
        return Some(("languages", "the site needs at least one language"));
    }
    if !file.languages.contains(&file.default_language) {
        return Some((
            "default_language",
            "the default language must be one of languages",
        ));
    }
    let bar = file.announcement.iter().filter_map(|a| a.language.as_ref());
    let unknown = (file.nav.languages().chain(bar)).any(|l| !file.languages.contains(l));
    if unknown {
        let problem = "a menu link or the announcement names a language the site does not have; use one of languages";
        return Some(("language", problem));
    }
    None
}

/// The refusal of `[code]` that waits for consent on a static site, at the
/// consent line when it is written, else at the table's own line.
fn static_consent(path: &Path, text: &str) -> ConfigError {
    let table = text
        .lines()
        .position(|line| line.trim_start().starts_with("[code]"))
        .map_or(1, |index| index.saturating_add(1));
    ConfigError::Field {
        path: path.to_path_buf(),
        line: line_of_key_in(text, "[code", "consent").max(table),
        field: "consent".to_owned(),
        problem: "a static host cannot keep a reader's choice, so code that waits for consent would never run; set consent = \"not-needed\" in [code], or mode = \"server\"".to_owned(),
    }
}

impl Config {
    /// The settings file's name inside a site folder.
    pub const FILE_NAME: &str = "site.toml";

    /// Reads `site.toml` from the `site` folder and applies `env` on top.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Read`] when the file cannot be read, and the
    /// errors of [`Config::from_toml`] when its content is wrong.
    pub fn load(site: &Path, env: &Environment) -> Result<Self, ConfigError> {
        let mut path = site.to_path_buf();
        path.push(Self::FILE_NAME);
        let text = fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;
        Self::from_toml(&path, &text, env)
    }

    /// Checks the text of a settings file, named `path` in messages, and
    /// applies `env` on top.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] or [`ConfigError::Field`] with the line
    /// and field of the first mistake, and [`ConfigError::Environment`] when an
    /// environment variable holds a value that is not allowed.
    pub fn from_toml(path: &Path, text: &str, env: &Environment) -> Result<Self, ConfigError> {
        let file: SiteFile = toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            line: line_of(text, source.span().map_or(0, |span| span.start)),
            field: field_of(text, &source),
            source: Box::new(source),
        })?;
        let field = |field: &str, problem: &str| ConfigError::Field {
            path: path.to_path_buf(),
            line: line_of_key(text, field),
            field: field.to_owned(),
            problem: problem.to_owned(),
        };
        if let Some((key, problem)) = layout_mistake(&file) {
            return Err(field(key, problem));
        }
        let preview_link_lifetime =
            preview_lifetime(&file).map_err(|(key, problem)| field(key, problem))?;
        if let Some(s3) = &file.media.s3 {
            if let Some((key, problem)) = s3.mistake() {
                return Err(field(key, problem));
            }
            if env.s3_keys().is_none() {
                let missing = "a bucket needs KHALID_S3_ACCESS_KEY_ID and KHALID_S3_SECRET_ACCESS_KEY set in the environment";
                return Err(field("bucket", missing));
            }
        }
        let in_table = |table: &str, key: &str, problem: &str| ConfigError::Field {
            path: path.to_path_buf(),
            line: line_of_key_in(text, table, key),
            field: key.to_owned(),
            problem: problem.to_owned(),
        };
        let mail = file.mail.map(|table| table.check(env)).transpose();
        let mail = mail.map_err(|(key, problem)| in_table("[mail", key, problem))?;
        if let Some((key, problem)) = file.alerts.mistake(mail.as_ref()) {
            return Err(in_table("[alerts", key, problem));
        }
        if file.mode == Mode::Static && file.code.waits_for_consent() {
            return Err(static_consent(path, text));
        }
        let address = match env.get(Environment::ADDRESS) {
            Some(text) => AbsoluteUrl::parse(text).map_err(|source| ConfigError::Environment {
                variable: Environment::ADDRESS.to_owned(),
                source,
            })?,
            None => file.address,
        };
        let search_console = file
            .search_console
            .map(|table| table.check(&address, mail.as_ref()));
        let search_console = search_console
            .transpose()
            .map_err(|(key, problem)| in_table("[search_console", key, problem))?;
        Ok(Self {
            name: file.name,
            address,
            mount: file.mount,
            mode: file.mode,
            slugs: file.slugs,
            default_language: file.default_language,
            languages: file.languages,
            seo: file.seo,
            reader: file.reader,
            nav: file.nav,
            announcement: file.announcement,
            media: file.media,
            mail,
            alerts: file.alerts,
            code: file.code,
            search_console,
            preview_link_lifetime,
        })
    }

    /// When alerts are raised and who gets them by email, from `[alerts]`.
    pub fn alerts(&self) -> &AlertSettings {
        &self.alerts
    }

    /// The site's own code for every page, from `[code]`; empty when the
    /// site has none.
    pub fn code(&self) -> &SiteCode {
        &self.code
    }

    /// How the site sends email, from `[mail]`, or `None` when it sends
    /// none.
    pub fn mail(&self) -> Option<&MailSettings> {
        self.mail.as_ref()
    }

    /// The Search Console connection, from `[search_console]`, or `None`
    /// when the owner has not opted in.
    pub fn search_console(&self) -> Option<&SearchConsoleSettings> {
        self.search_console.as_ref()
    }

    /// Where the media library keeps its files, from `[media]`.
    pub fn media(&self) -> &MediaSettings {
        &self.media
    }

    /// How long a new preview link works: 7 days unless `site.toml` sets
    /// `preview_link_days` or `preview_link_hours`, from 1 hour to 30 days.
    pub fn preview_link_lifetime(&self) -> Duration {
        self.preview_link_lifetime
    }

    /// The site's name, shown in titles and feeds.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the site lives, such as `https://www.example.com/`.
    pub fn address(&self) -> &AbsoluteUrl {
        &self.address
    }

    /// The folder the blog sits under on that site, such as `/blog/`.
    pub fn mount(&self) -> &str {
        &self.mount
    }

    /// Server, static or headless.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// How slugs are written.
    pub fn slugs(&self) -> SlugMode {
        self.slugs
    }

    /// The language a page is in when nothing says otherwise.
    pub fn default_language(&self) -> &Locale {
        &self.default_language
    }

    /// Every language the site publishes in, in the order written.
    pub fn languages(&self) -> &[Locale] {
        &self.languages
    }

    /// The SEO settings, from `[seo]`.
    pub fn seo(&self) -> &SeoSettings {
        &self.seo
    }

    /// Which reader features the default templates show, from `[reader]`.
    pub fn reader(&self) -> &Reader {
        &self.reader
    }

    /// The header and footer menus, from `[nav]`.
    pub fn nav(&self) -> &NavSettings {
        &self.nav
    }

    /// The line shown above every page's header, from `[announcement]`, or
    /// `None` when the site has none.
    pub fn announcement(&self) -> Option<&Announcement> {
        self.announcement.as_ref()
    }
}

/// Why the settings could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The settings file could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The file is not valid, or a field has the wrong kind of value.
    #[error("{}, line {line}, field {field:?}: {}", path.display(), source.message())]
    Parse {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the mistake.
        line: usize,
        /// The field, or empty when the mistake is not in one field.
        field: String,
        /// What the TOML reader found.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A field is readable but breaks a rule that spans the file.
    #[error("{}, line {line}, field {field:?}: {problem}", path.display())]
    Field {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the field.
        line: usize,
        /// The field.
        field: String,
        /// The rule it breaks.
        problem: String,
    },
    /// An environment variable holds a value that is not allowed.
    #[error("the environment variable {variable} is not a valid address")]
    Environment {
        /// The variable.
        variable: String,
        /// Why its value was refused.
        #[source]
        source: ParseUrlError,
    },
}
