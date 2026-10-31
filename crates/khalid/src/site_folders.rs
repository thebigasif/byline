//! The sites in a sites folder as the super-admin panel and `khalid sites`
//! see them: `sites.toml` and one site folder per domain (plan 4.4).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use khalid_api::content_store::BoxFuture;
use khalid_auth::{AddSiteError, Email, Home, ListedSite, SiteDirectory, SiteDirectoryError};
use khalid_core::{AbsoluteUrl, DomainName, UserMessage};
use tokio::sync::{Mutex, mpsc};

use crate::site_list::{self, SiteList};
use crate::{init, sites_message};

/// The sites folder at `root`. Adding a site makes its folder, lists it,
/// then tells `started`, so a running server starts serving it.
#[derive(Debug, Clone)]
pub(crate) struct SiteFolders {
    root: PathBuf,
    adding: Arc<Mutex<()>>,
    started: Option<mpsc::UnboundedSender<DomainName>>,
}

/// Why a site was not added.
#[derive(Debug)]
pub(crate) enum AddError {
    /// A site already answers on the domain.
    Taken,
    /// The folder, its list or the new site could not be read or made.
    Failed(UserMessage),
}

impl std::fmt::Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Taken => f.write_str("a site already answers on this domain"),
            Self::Failed(message) => f.write_str(message.what()),
        }
    }
}

impl std::error::Error for AddError {}

impl SiteFolders {
    /// The sites folder at `root`, telling `started` of each site added.
    pub(crate) fn new(root: &Path, started: Option<mpsc::UnboundedSender<DomainName>>) -> Self {
        Self {
            root: root.to_path_buf(),
            adding: Arc::new(Mutex::new(())),
            started,
        }
    }

    /// The list, read on a blocking thread (RULES.md BL-RUN-7).
    async fn list(&self) -> Result<SiteList, SiteDirectoryError> {
        let root = self.root.clone();
        let read = tokio::task::spawn_blocking(move || listed(&root)).await;
        read.map_err(SiteDirectoryError::new)?
            .map_err(SiteDirectoryError::new)
    }
}

/// The list in `root`, which must have one.
fn listed(root: &Path) -> Result<SiteList, AddError> {
    match SiteList::read(root) {
        Ok(Some(list)) => Ok(list),
        Ok(None) => Err(AddError::Failed(sites_message::unreadable(
            &site_list::file(root),
        ))),
        Err(message) => Err(AddError::Failed(message)),
    }
}

/// Adds a site on `domain` to the sites folder `root`: makes its folder
/// with the usual settings, keeping one already there, then lists it. A
/// folder without `sites.toml` becomes a sites folder with this site as
/// its home.
///
/// # Errors
///
/// Returns [`AddError::Taken`] when the domain is listed, and
/// [`AddError::Failed`] when the list or the folder cannot be made.
pub(crate) fn add(root: &Path, domain: &DomainName) -> Result<SiteList, AddError> {
    let list = match SiteList::read(root).map_err(AddError::Failed)? {
        Some(mut list) => {
            list.with_site(domain.clone())
                .map_err(|_| AddError::Taken)?;
            list
        }
        None => SiteList::new(domain.clone()),
    };
    let folder = site_list::folder(root, domain);
    let address = AbsoluteUrl::parse(&format!("https://{}/", domain.as_str()))
        .map_err(|_| AddError::Failed(sites_message::bad_domain(root, domain.as_str())))?;
    let failed = |_| AddError::Failed(sites_message::not_made(&folder));
    init::make_site(&folder, address).map_err(failed)?;
    list.write(root).map_err(failed)?;
    Ok(list)
}

/// The directory of a site with no super admin: one served alone, or one
/// among many that is not the home site. Its panel routes refuse everyone.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoPanel;

impl SiteDirectory for NoPanel {
    fn sites(&self) -> BoxFuture<'_, Result<Vec<ListedSite>, SiteDirectoryError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn add<'a>(
        &'a self,
        _domain: &'a DomainName,
    ) -> BoxFuture<'a, Result<ListedSite, AddSiteError>> {
        Box::pin(async { Err(unavailable(AddError::Failed(sites_message::no_panel()))) })
    }

    fn is_super_admin<'a>(
        &'a self,
        _email: &'a Email,
    ) -> BoxFuture<'a, Result<bool, SiteDirectoryError>> {
        Box::pin(async { Ok(false) })
    }
}

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> AddSiteError {
    AddSiteError::Unavailable(SiteDirectoryError::new(error))
}

impl SiteDirectory for SiteFolders {
    fn sites(&self) -> BoxFuture<'_, Result<Vec<ListedSite>, SiteDirectoryError>> {
        Box::pin(async move {
            let list = self.list().await?;
            let homes = std::iter::once(Home::Yes).chain(std::iter::repeat(Home::No));
            let domains = list.domains().iter().cloned().zip(homes);
            Ok(domains
                .map(|(domain, home)| ListedSite::new(domain, home))
                .collect())
        })
    }

    fn add<'a>(
        &'a self,
        domain: &'a DomainName,
    ) -> BoxFuture<'a, Result<ListedSite, AddSiteError>> {
        Box::pin(async move {
            let _one_at_a_time = self.adding.lock().await;
            let (root, owned) = (self.root.clone(), domain.clone());
            let added = tokio::task::spawn_blocking(move || {
                listed(&root)?;
                add(&root, &owned)
            })
            .await;
            match added {
                Ok(Ok(_)) => {}
                Ok(Err(AddError::Taken)) => return Err(AddSiteError::Taken),
                Ok(Err(error)) => return Err(unavailable(error)),
                Err(error) => return Err(unavailable(error)),
            }
            if let Some(started) = &self.started {
                // A server that stopped listening has nothing left to start.
                let _ = started.send(domain.clone());
            }
            Ok(ListedSite::new(domain.clone(), Home::No))
        })
    }

    fn is_super_admin<'a>(
        &'a self,
        email: &'a Email,
    ) -> BoxFuture<'a, Result<bool, SiteDirectoryError>> {
        Box::pin(async move { Ok(self.list().await?.is_super_admin(email)) })
    }
}

#[cfg(test)]
mod tests {
    use khalid_auth::site_domain;
    use khalid_core::Config;

    use super::*;

    #[tokio::test]
    async fn an_added_site_gets_a_folder_a_line_in_the_list_and_a_start() {
        let root = tempfile::tempdir().unwrap();
        let home = site_domain("example.com").unwrap();
        let mut list = add(root.path(), &home).unwrap();
        list.with_super_admin("owner@example.com".parse().unwrap());
        list.write(root.path()).unwrap();
        let (started, mut starts) = mpsc::unbounded_channel();
        let folders = SiteFolders::new(root.path(), Some(started));

        let news = site_domain("news.example.com").unwrap();
        assert_eq!(
            SiteDirectory::add(&folders, &news).await.unwrap().domain,
            news
        );
        assert_eq!(starts.try_recv().unwrap(), news);
        let mut settings = site_list::folder(root.path(), &news);
        settings.push(Config::FILE_NAME);
        let text = std::fs::read_to_string(settings).unwrap();
        assert!(
            text.contains("address = \"https://news.example.com/\""),
            "{text}"
        );

        let sites = folders.sites().await.unwrap();
        let want = vec![
            ListedSite::new(home.clone(), Home::Yes),
            ListedSite::new(news.clone(), Home::No),
        ];
        assert_eq!(sites, want);
        let again = SiteDirectory::add(&folders, &home).await;
        assert!(matches!(again, Err(AddSiteError::Taken)), "{again:?}");
        assert!(starts.try_recv().is_err());

        let owner = "owner@example.com".parse().unwrap();
        let stranger = "someone@example.com".parse().unwrap();
        assert!(folders.is_super_admin(&owner).await.unwrap());
        assert!(!folders.is_super_admin(&stranger).await.unwrap());
    }

    #[tokio::test]
    async fn the_panel_adds_nothing_to_a_folder_that_is_not_a_sites_folder() {
        let root = tempfile::tempdir().unwrap();
        let folders = SiteFolders::new(root.path(), None);
        let news = site_domain("news.example.com").unwrap();
        let added = SiteDirectory::add(&folders, &news).await;
        assert!(
            matches!(added, Err(AddSiteError::Unavailable(_))),
            "{added:?}"
        );
        assert!(!site_list::file(root.path()).exists());
        let email = "owner@example.com".parse().unwrap();
        assert!(folders.is_super_admin(&email).await.is_err());
    }
}
