//! `sites.toml`: the sites one process serves and its super admins (plan
//! section 4.4, ADR "many sites in one process"). A folder holding this
//! file is a sites folder; each site lives in `sites/<domain>/` beside it,
//! a site folder like any other. The first site is the home site, where
//! the super-admin panel lives.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use khalid_auth::{Email, site_domain};
use khalid_core::{DomainName, UserMessage};
use serde::Deserialize;

use crate::init::quoted;
use crate::{atomic, sites_message};

/// The file that makes a folder a sites folder.
pub(crate) const FILE: &str = "sites.toml";

/// The folder beside it that holds one folder per site.
const FOLDER: &str = "sites";

/// The sites in a sites folder: never empty, each domain once, the home
/// site first; and the emails of its super admins, each once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteList {
    domains: Vec<DomainName>,
    super_admins: Vec<Email>,
}

/// `sites.toml` as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SitesFile {
    #[serde(default)]
    super_admins: Vec<String>,
    #[serde(default)]
    site: Vec<SiteRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SiteRow {
    domain: String,
}

impl SiteList {
    /// A list with the home site on `home` and no super admin yet.
    pub(crate) fn new(home: DomainName) -> Self {
        Self {
            domains: vec![home],
            super_admins: Vec::new(),
        }
    }

    /// Reads `sites.toml` in `root`, or `None` when `root` has none.
    ///
    /// # Errors
    ///
    /// Returns a message when the file cannot be read or is not a list.
    pub(crate) fn read(root: &Path) -> Result<Option<Self>, UserMessage> {
        let path = file(root);
        match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&path, &text).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(sites_message::unreadable(&path)),
        }
    }

    /// Reads `text`, the contents of the file at `path`.
    fn parse(path: &Path, text: &str) -> Result<Self, UserMessage> {
        let written: SitesFile =
            toml::from_str(text).map_err(|error| sites_message::invalid(path, error.message()))?;
        let mut list: Option<Self> = None;
        for row in written.site {
            let domain = site_domain(&row.domain)
                .map_err(|given| sites_message::bad_domain(path, &given))?;
            match &mut list {
                None => list = Some(Self::new(domain)),
                Some(list) => list
                    .with_site(domain)
                    .map_err(|domain| sites_message::twice(path, domain.as_str()))?,
            }
        }
        let mut list = list.ok_or_else(|| sites_message::no_site(path))?;
        for email in written.super_admins {
            let email: Email = email
                .parse()
                .map_err(|_| sites_message::bad_email(path, &email))?;
            list.with_super_admin(email);
        }
        Ok(list)
    }

    /// Writes the list to `sites.toml` in `root`, replacing it whole.
    ///
    /// # Errors
    ///
    /// Returns the error when the file cannot be written.
    pub(crate) fn write(&self, root: &Path) -> io::Result<()> {
        atomic::replace(&file(root), &self.to_toml())
    }

    /// The list as `sites.toml` text.
    fn to_toml(&self) -> String {
        let admins: Vec<String> = self
            .super_admins
            .iter()
            .map(|email| quoted(email.as_str()))
            .collect();
        let sites: Vec<String> = self
            .domains
            .iter()
            .map(|domain| format!("\n[[site]]\ndomain = {}\n", quoted(domain.as_str())))
            .collect();
        format!(
            "# The sites this process serves. The first is the home site, where the\n\
             # super-admin panel lives. Each site is a folder in sites/.\n\n\
             super_admins = [{}]\n{}",
            admins.join(", "),
            sites.concat()
        )
    }

    /// Every site's domain, the home site first.
    pub(crate) fn domains(&self) -> &[DomainName] {
        &self.domains
    }

    /// Adds a site on `domain` after the others.
    ///
    /// # Errors
    ///
    /// Returns `domain` back when a site already has it.
    pub(crate) fn with_site(&mut self, domain: DomainName) -> Result<(), DomainName> {
        if self.domains.contains(&domain) {
            return Err(domain);
        }
        self.domains.push(domain);
        Ok(())
    }

    /// Lists `email` as a super admin, once.
    pub(crate) fn with_super_admin(&mut self, email: Email) {
        if !self.super_admins.contains(&email) {
            self.super_admins.push(email);
        }
    }

    /// Whether `email` is a super admin.
    pub(crate) fn is_super_admin(&self, email: &Email) -> bool {
        self.super_admins.contains(email)
    }
}

/// `sites.toml` in `root`.
pub(crate) fn file(root: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(FILE);
    path
}

/// The folder of the site on `domain` in the sites folder `root`.
pub(crate) fn folder(root: &Path, domain: &DomainName) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(FOLDER);
    path.push(domain.as_str());
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(text: &str) -> DomainName {
        site_domain(text).unwrap()
    }

    #[test]
    fn a_list_reads_back_what_it_writes() {
        let mut list = SiteList::new(domain("example.com"));
        list.with_site(domain("news.example.com")).unwrap();
        list.with_super_admin("owner@example.com".parse().unwrap());
        list.with_super_admin("owner@example.com".parse().unwrap());
        let text = list.to_toml();
        assert_eq!(SiteList::parse(Path::new(FILE), &text).unwrap(), list);
        assert_eq!(text.matches("owner@example.com").count(), 1, "{text}");
        assert!(list.with_site(domain("example.com")).is_err());
    }

    #[test]
    fn a_list_that_cannot_be_served_is_refused_with_the_reason() {
        let refused = [
            ("super_admins = []", "no site"),
            ("[[site]]\ndomain = \"../etc\"", "../etc"),
            (
                "[[site]]\ndomain = \"a.com\"\n[[site]]\ndomain = \"A.com\"",
                "twice",
            ),
            (
                "super_admins = [\"nobody\"]\n[[site]]\ndomain = \"a.com\"",
                "nobody",
            ),
            ("[[site]]\nhost = \"a.com\"", "host"),
        ];
        for (text, said) in refused {
            let message = SiteList::parse(Path::new(FILE), text).unwrap_err();
            let shown = format!("{} {} {}", message.what(), message.why(), message.fix());
            assert!(shown.contains(said), "{text}: {shown}");
        }
    }

    #[test]
    fn each_site_has_its_own_folder_under_sites() {
        let path = folder(Path::new("root"), &domain("news.example.com"));
        assert_eq!(path, Path::new("root/sites/news.example.com"));
    }
}
