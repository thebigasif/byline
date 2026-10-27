//! [`LinkSettings`]: what links to other sites get, from the `[seo.links]`
//! table of `site.toml`.

use std::fmt;

use serde::Deserialize;

/// What every link to another site gets in the blog's posts and pages. By
/// default it gets nothing added and opens in the same tab:
///
/// ```toml
/// [seo.links]
/// rel = ["nofollow"]
/// open = "new-tab"
/// affiliate_domains = ["amazon.com"]
/// internal_domains = ["shop.example.com"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct LinkSettings {
    /// The `rel` values every external link gets.
    pub rel: Vec<ExternalRel>,
    /// Where external links open.
    pub open: LinkOpening,
    /// Domains whose links are paid or affiliate links, so always
    /// `sponsored`, with their subdomains.
    pub affiliate_domains: Vec<DomainName>,
    /// Other domains that count as this site, such as the main site's shop.
    pub internal_domains: Vec<DomainName>,
}

/// A `rel` value a site can put on every external link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalRel {
    /// `nofollow`: the site does not vouch for the page.
    NoFollow,
    /// `sponsored`: the link is paid.
    Sponsored,
    /// `ugc`: a visitor wrote the link.
    Ugc,
}

/// Where an external link opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkOpening {
    /// In the same tab.
    #[default]
    SameTab,
    /// In a new tab.
    NewTab,
}

/// A domain name such as `amazon.com`, in lowercase: ASCII letters, digits,
/// hyphens and dots, neither starting nor ending with a dot.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainName(String);

impl DomainName {
    /// Checks `text` and keeps it in lowercase.
    ///
    /// # Errors
    ///
    /// Returns [`ParseDomainNameError`] when `text` is empty, starts or ends
    /// with a dot, or holds anything but ASCII letters, digits, `-` and `.`.
    pub fn try_new(text: &str) -> Result<Self, ParseDomainNameError> {
        let valid = !text.is_empty()
            && !text.starts_with('.')
            && !text.ends_with('.')
            && text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'));
        if valid {
            Ok(Self(text.to_ascii_lowercase()))
        } else {
            Err(ParseDomainNameError {
                text: text.to_owned(),
            })
        }
    }

    /// The name in lowercase.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DomainName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DomainName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::try_new(&text).map_err(serde::de::Error::custom)
    }
}

/// Why a text is not a [`DomainName`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{text}` is not a domain name, expected a name such as amazon.com, with no https:// and no path"
)]
pub struct ParseDomainNameError {
    text: String,
}
