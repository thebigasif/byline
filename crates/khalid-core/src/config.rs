//! Settings, read once when Khalid starts.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::mail_settings::MailTable;
use crate::toml_location::{field_of, line_of, line_of_key, line_of_key_in};
use crate::{
    AbsoluteUrl, AlertSettings, Announcement, Locale, MailSettings, MailTransport, MediaSettings,
    NavSettings, ParseUrlError, Reader, S3Keys, Secret, SeoSettings, SiteCode,
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
    preview_link_days: Option<u64>,
    preview_link_hours: Option<u64>,
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
        let mount = &file.mount;
        if !mount.starts_with('/') || !mount.ends_with('/') || mount.contains("//") {
            return Err(field(
                "mount",
                "the mount path must start and end with /, like /blog/",
            ));
        }
        if file.languages.is_empty() {
            return Err(field("languages", "the site needs at least one language"));
        }
        if !file.languages.contains(&file.default_language) {
            return Err(field(
                "default_language",
                "the default language must be one of languages",
            ));
        }
        let bar = file.announcement.iter().filter_map(|a| a.language.as_ref());
        let unknown = (file.nav.languages().chain(bar)).any(|l| !file.languages.contains(l));
        if unknown {
            let problem = "a menu link or the announcement names a language the site does not have; use one of languages";
            return Err(field("language", problem));
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

/// The environment variables Khalid reads, captured once at start. `Debug`
/// names the variables and hides the key's value (RULES.md BL-RUN-5).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Environment(BTreeMap<String, String>);

impl Environment {
    /// Overrides `address` in `site.toml`, for deploys that move the site.
    pub const ADDRESS: &str = "KHALID_ADDRESS";

    /// The integration key or staff token `khalid mcp` acts with. An agent's
    /// settings set it, so the key never appears on a command line.
    pub const KEY: &str = "KHALID_KEY";

    /// The identity of the key that writes to the media bucket.
    pub const S3_ACCESS_KEY_ID: &str = "KHALID_S3_ACCESS_KEY_ID";

    /// The secret of the key that writes to the media bucket.
    pub const S3_SECRET_ACCESS_KEY: &str = "KHALID_S3_SECRET_ACCESS_KEY";

    /// The password of the SMTP user in `[mail.smtp]`.
    pub const SMTP_PASSWORD: &str = "KHALID_SMTP_PASSWORD";

    /// The API key that sends mail through Resend.
    pub const RESEND_API_KEY: &str = "KHALID_RESEND_API_KEY";

    /// The server token that sends mail through Postmark.
    pub const POSTMARK_TOKEN: &str = "KHALID_POSTMARK_TOKEN";

    /// The Bing Webmaster Tools API key, made there under Settings, API
    /// Access. It belongs to a person, not a site, and reads every site they
    /// have verified there.
    pub const BING_API_KEY: &str = "KHALID_BING_API_KEY";

    /// Every variable Khalid reads.
    const ALL: [&str; 8] = [
        Self::ADDRESS,
        Self::KEY,
        Self::S3_ACCESS_KEY_ID,
        Self::S3_SECRET_ACCESS_KEY,
        Self::SMTP_PASSWORD,
        Self::RESEND_API_KEY,
        Self::POSTMARK_TOKEN,
        Self::BING_API_KEY,
    ];

    /// Variables whose values `Debug` hides.
    const HIDDEN: [&str; 6] = [
        Self::KEY,
        Self::S3_SECRET_ACCESS_KEY,
        Self::SMTP_PASSWORD,
        Self::RESEND_API_KEY,
        Self::POSTMARK_TOKEN,
        Self::BING_API_KEY,
    ];

    /// Reads Khalid's variables from this process's environment.
    pub fn from_process() -> Self {
        let mut found = BTreeMap::new();
        for name in Self::ALL {
            #[expect(
                clippy::disallowed_methods,
                reason = "the one place that reads the environment (BL-RUN-2)"
            )]
            let value = std::env::var(name);
            if let Ok(value) = value {
                found.insert(name.to_owned(), value);
            }
        }
        Self(found)
    }

    /// An environment with no variables set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// An environment with exactly these variables, for tests and tools.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        )
    }

    /// The key in [`Environment::KEY`], or `None` when it is unset or blank.
    pub fn key(&self) -> Option<Secret<String>> {
        self.get(Self::KEY)
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(|key| Secret::new(key.to_owned()))
    }

    /// The media bucket's two keys, or `None` unless both are set and not
    /// blank.
    pub fn s3_keys(&self) -> Option<S3Keys> {
        let read = |name| {
            self.get(name)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Secret::new(value.to_owned()))
        };
        Some(S3Keys {
            access_key_id: read(Self::S3_ACCESS_KEY_ID)?,
            secret_access_key: read(Self::S3_SECRET_ACCESS_KEY)?,
        })
    }

    /// The password, key or token `transport` signs in with, or `None`
    /// when it is unset or blank.
    pub fn mail_secret(&self, transport: &MailTransport) -> Option<Secret<String>> {
        let name = match transport {
            MailTransport::Smtp(_) => Self::SMTP_PASSWORD,
            MailTransport::Resend => Self::RESEND_API_KEY,
            MailTransport::Postmark(_) => Self::POSTMARK_TOKEN,
        };
        self.get(name)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Secret::new(value.to_owned()))
    }

    /// The key in [`Environment::BING_API_KEY`], or `None` when it is unset
    /// or blank.
    pub fn bing_api_key(&self) -> Option<Secret<String>> {
        self.get(Self::BING_API_KEY)
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(|key| Secret::new(key.to_owned()))
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

impl fmt::Debug for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = self.0.iter().map(|(name, value)| {
            let value: &dyn fmt::Debug = if Self::HIDDEN.contains(&name.as_str()) {
                &"[redacted]"
            } else {
                value
            };
            (name, value)
        });
        f.debug_map().entries(shown).finish()
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
