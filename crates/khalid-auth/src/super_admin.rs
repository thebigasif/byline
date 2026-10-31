//! The sites one process serves and who may add to them: the super admin
//! of plan section 4.4 (PRD KH-EDT-7).
//!
//! A super admin is a person, never a key. They sign in on the home site,
//! the one the super-admin panel lives on, the way any staff member does,
//! and the process lists their email as a super admin. The binary keeps
//! the list and the sites behind [`SiteDirectory`].

use std::error::Error;

use khalid_api::content_store::BoxFuture;
use khalid_core::DomainName;

use crate::Email;

/// The longest domain name DNS can hold, in characters (RFC 1035).
const MAX_DOMAIN: usize = 253;

/// One site the process serves.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedSite {
    /// The domain readers reach it on.
    pub domain: DomainName,
    /// Whether it is the home site, where the super-admin panel lives.
    pub home: Home,
}

impl ListedSite {
    /// A site on `domain`.
    #[must_use]
    pub fn new(domain: DomainName, home: Home) -> Self {
        Self { domain, home }
    }
}

/// Whether a site is the home site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// The home site, where the super-admin panel lives.
    Yes,
    /// Another site.
    No,
}

/// The sites a process serves and its super admins, kept by the binary.
pub trait SiteDirectory: Send + Sync {
    /// Every site, the home site first.
    ///
    /// # Errors
    ///
    /// Returns [`SiteDirectoryError`] when the list cannot be read.
    fn sites(&self) -> BoxFuture<'_, Result<Vec<ListedSite>, SiteDirectoryError>>;

    /// Adds a site on `domain` and starts serving it.
    ///
    /// # Errors
    ///
    /// Returns [`AddSiteError::Taken`] when a site already answers on
    /// `domain`, and [`AddSiteError::Unavailable`] when it cannot be made.
    fn add<'a>(&'a self, domain: &'a DomainName)
    -> BoxFuture<'a, Result<ListedSite, AddSiteError>>;

    /// Whether the person with `email` on the home site is a super admin.
    ///
    /// # Errors
    ///
    /// Returns [`SiteDirectoryError`] when the list cannot be read.
    fn is_super_admin<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<bool, SiteDirectoryError>>;
}

/// The site list could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the site list could not be read or written")]
pub struct SiteDirectoryError(#[source] Box<dyn Error + Send + Sync>);

impl SiteDirectoryError {
    /// Wraps the cause.
    pub fn new(cause: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(cause))
    }
}

/// Why a site could not be added.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AddSiteError {
    /// A site already answers on this domain.
    #[error("a site already answers on this domain")]
    Taken,
    /// The site could not be made or started.
    #[error("the site could not be made or started")]
    Unavailable(#[source] SiteDirectoryError),
}

/// Reads a domain a super admin typed: a [`DomainName`] DNS can hold, with
/// no empty label, so it is also a safe folder name.
///
/// # Errors
///
/// Returns the text back when it is not such a name.
pub fn site_domain(text: &str) -> Result<DomainName, String> {
    let fits = text.len() <= MAX_DOMAIN && !text.contains("..");
    match DomainName::try_new(text) {
        Ok(domain) if fits => Ok(domain),
        _ => Err(text.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_domain_is_a_safe_folder_name() {
        assert_eq!(
            site_domain("News.Example.com").unwrap().as_str(),
            "news.example.com"
        );
        for refused in ["", ".", "..", "a..b", "../etc", "a/b", "a b", ".hidden"] {
            assert!(site_domain(refused).is_err(), "{refused:?}");
        }
        assert!(site_domain(&"a".repeat(254)).is_err());
    }
}
