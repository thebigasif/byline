//! The environment variables Khalid reads, captured once at start. Kept
//! apart from `config.rs` so that file stays under 600 lines (BL-SIZE-1).

use std::collections::BTreeMap;
use std::fmt;

use crate::{MailTransport, S3Keys, Secret};

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

    /// The secret of the OAuth client named in `[search_console]`.
    pub const GOOGLE_CLIENT_SECRET: &str = "KHALID_GOOGLE_CLIENT_SECRET";

    /// The refresh token Google gave when the owner let that client read
    /// their Search Console numbers.
    pub const GOOGLE_REFRESH_TOKEN: &str = "KHALID_GOOGLE_REFRESH_TOKEN";

    /// The access key of an Unsplash app, made at unsplash.com/developers,
    /// that photo search in the editor asks with.
    pub const UNSPLASH_ACCESS_KEY: &str = "KHALID_UNSPLASH_ACCESS_KEY";

    /// The API key of a GIPHY app, made at developers.giphy.com, that GIF
    /// search in the editor asks with.
    pub const GIPHY_API_KEY: &str = "KHALID_GIPHY_API_KEY";

    /// Every variable Khalid reads.
    const ALL: [&str; 12] = [
        Self::ADDRESS,
        Self::KEY,
        Self::S3_ACCESS_KEY_ID,
        Self::S3_SECRET_ACCESS_KEY,
        Self::SMTP_PASSWORD,
        Self::RESEND_API_KEY,
        Self::POSTMARK_TOKEN,
        Self::BING_API_KEY,
        Self::GOOGLE_CLIENT_SECRET,
        Self::GOOGLE_REFRESH_TOKEN,
        Self::UNSPLASH_ACCESS_KEY,
        Self::GIPHY_API_KEY,
    ];

    /// Variables whose values `Debug` hides.
    const HIDDEN: [&str; 10] = [
        Self::KEY,
        Self::S3_SECRET_ACCESS_KEY,
        Self::SMTP_PASSWORD,
        Self::RESEND_API_KEY,
        Self::POSTMARK_TOKEN,
        Self::BING_API_KEY,
        Self::GOOGLE_CLIENT_SECRET,
        Self::GOOGLE_REFRESH_TOKEN,
        Self::UNSPLASH_ACCESS_KEY,
        Self::GIPHY_API_KEY,
    ];

    /// Reads Khalid's variables from this process's environment.
    pub fn from_process() -> Self {
        let mut found = BTreeMap::new();
        for name in Self::ALL {
            if let Some(value) = crate::config::process_variable(name) {
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
        self.secret(Self::BING_API_KEY)
    }

    /// The secret in [`Environment::GOOGLE_CLIENT_SECRET`], or `None` when
    /// it is unset or blank.
    pub fn google_client_secret(&self) -> Option<Secret<String>> {
        self.secret(Self::GOOGLE_CLIENT_SECRET)
    }

    /// The token in [`Environment::GOOGLE_REFRESH_TOKEN`], or `None` when
    /// it is unset or blank.
    pub fn google_refresh_token(&self) -> Option<Secret<String>> {
        self.secret(Self::GOOGLE_REFRESH_TOKEN)
    }

    /// The key in [`Environment::UNSPLASH_ACCESS_KEY`], or `None` when it is
    /// unset or blank.
    pub fn unsplash_key(&self) -> Option<Secret<String>> {
        self.secret(Self::UNSPLASH_ACCESS_KEY)
    }

    /// The key in [`Environment::GIPHY_API_KEY`], or `None` when it is unset
    /// or blank.
    pub fn giphy_key(&self) -> Option<Secret<String>> {
        self.secret(Self::GIPHY_API_KEY)
    }

    /// The variable `name`, trimmed, as a secret, or `None` when it is
    /// unset or blank.
    fn secret(&self, name: &str) -> Option<Secret<String>> {
        self.get(name)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Secret::new(value.to_owned()))
    }

    /// The variable `name` as set, or `None` when it is unset.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
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
