//! The opt-in Search Console connection, from the `[search_console]` table
//! of `site.toml` (KH-GRO-5, plan 5.11).

use serde::Deserialize;

use crate::mail_settings::is_address;
use crate::{AbsoluteUrl, MailSettings};

/// Which Google Search Console property the site's numbers come from, and
/// who gets the weekly report. Leave the table out and Khalid never asks
/// Google anything:
///
/// ```toml
/// [search_console]
/// client_id = "1234-abcd.apps.googleusercontent.com"
/// property = "sc-domain:example.com"
/// report_to = "owner@example.com"
/// ```
///
/// The client's secret and the refresh token that lets Khalid read the
/// owner's numbers never go in the file: they come from the environment,
/// see [`crate::Environment::google_client_secret`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchConsoleSettings {
    /// The OAuth client's identity, made in the owner's Google Cloud
    /// project, such as `1234-abcd.apps.googleusercontent.com`.
    pub client_id: String,
    /// The property, the site's address unless `property` names another.
    pub property: SearchConsoleProperty,
    /// Who gets the weekly email, or `None` for no email.
    pub report_to: Option<String>,
}

/// A Search Console property: a whole domain, or every address under a
/// prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SearchConsoleProperty {
    /// A domain property, written `sc-domain:example.com`.
    Domain(String),
    /// A URL-prefix property, written as its address, such as
    /// `https://www.example.com/`.
    UrlPrefix(AbsoluteUrl),
}

impl SearchConsoleProperty {
    /// The property as Google names it, such as `sc-domain:example.com`.
    pub fn as_str(&self) -> String {
        match self {
            Self::Domain(domain) => format!("sc-domain:{domain}"),
            Self::UrlPrefix(url) => url.as_str().to_owned(),
        }
    }

    /// The property written as `text`, or `None` when Google could not
    /// name it so.
    fn parse(text: &str) -> Option<Self> {
        if let Some(domain) = text.strip_prefix("sc-domain:") {
            let plain = |c: char| c.is_alphanumeric() || c == '-' || c == '.';
            let fits = domain.contains('.') && domain.chars().all(plain);
            return fits.then(|| Self::Domain(domain.to_owned()));
        }
        AbsoluteUrl::parse(text).ok().map(Self::UrlPrefix)
    }
}

/// `[search_console]` as written, before the checks.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchConsoleTable {
    client_id: String,
    property: Option<String>,
    report_to: Option<String>,
}

impl SearchConsoleTable {
    /// The checked settings for the site at `address`, which sends mail
    /// the way `mail` says, or the key a mistake is under and what is
    /// wrong.
    pub(crate) fn check(
        self,
        address: &AbsoluteUrl,
        mail: Option<&MailSettings>,
    ) -> Result<SearchConsoleSettings, (&'static str, &'static str)> {
        let id = self.client_id.trim();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_graphic()) {
            return Err((
                "client_id",
                "client_id is the OAuth client ID from Google Cloud, such as 1234-abcd.apps.googleusercontent.com",
            ));
        }
        let property = match self.property.as_deref() {
            None => SearchConsoleProperty::UrlPrefix(address.clone()),
            Some(text) => SearchConsoleProperty::parse(text).ok_or((
                "property",
                "property is written as Search Console shows it, such as sc-domain:example.com or https://www.example.com/",
            ))?,
        };
        if let Some(to) = &self.report_to {
            if !is_address(to) {
                return Err((
                    "report_to",
                    "report_to is the address the weekly email goes to, such as owner@example.com",
                ));
            }
            if mail.is_none() {
                return Err((
                    "report_to",
                    "the weekly email needs [mail] in site.toml, which says how mail leaves",
                ));
            }
        }
        Ok(SearchConsoleSettings {
            client_id: id.to_owned(),
            property,
            report_to: self.report_to,
        })
    }
}
