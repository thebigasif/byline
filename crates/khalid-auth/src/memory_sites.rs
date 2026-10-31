//! A [`SiteDirectory`] in memory, for tests.

use std::sync::{Mutex, PoisonError};

use khalid_api::content_store::BoxFuture;
use khalid_core::DomainName;

use crate::{AddSiteError, Email, Home, ListedSite, SiteDirectory, SiteDirectoryError};

/// Sites and super admins held in memory; the first site is home.
#[derive(Debug)]
pub struct MemorySites {
    sites: Mutex<Vec<DomainName>>,
    super_admins: Vec<Email>,
}

impl MemorySites {
    /// A process serving `home` alone, with `super_admins`.
    #[must_use]
    pub fn new(home: DomainName, super_admins: Vec<Email>) -> Self {
        Self {
            sites: Mutex::new(vec![home]),
            super_admins,
        }
    }

    fn listed(sites: &[DomainName]) -> Vec<ListedSite> {
        let homes = std::iter::once(Home::Yes).chain(std::iter::repeat(Home::No));
        sites
            .iter()
            .zip(homes)
            .map(|(domain, home)| ListedSite::new(domain.clone(), home))
            .collect()
    }
}

impl SiteDirectory for MemorySites {
    fn sites(&self) -> BoxFuture<'_, Result<Vec<ListedSite>, SiteDirectoryError>> {
        let sites = self.sites.lock().unwrap_or_else(PoisonError::into_inner);
        let listed = Self::listed(&sites);
        Box::pin(async move { Ok(listed) })
    }

    fn add<'a>(
        &'a self,
        domain: &'a DomainName,
    ) -> BoxFuture<'a, Result<ListedSite, AddSiteError>> {
        let mut sites = self.sites.lock().unwrap_or_else(PoisonError::into_inner);
        let added = if sites.contains(domain) {
            Err(AddSiteError::Taken)
        } else {
            sites.push(domain.clone());
            Ok(ListedSite::new(domain.clone(), Home::No))
        };
        Box::pin(async move { added })
    }

    fn is_super_admin<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<bool, SiteDirectoryError>> {
        let listed = self.super_admins.contains(email);
        Box::pin(async move { Ok(listed) })
    }
}
