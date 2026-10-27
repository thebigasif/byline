//! [`IndexNow`]: the key file and the requests that tell search engines
//! which addresses changed.

use std::collections::BTreeSet;

use khalid_core::{AbsoluteUrl, IndexNowKey, IndexNowSettings};
use serde_json::json;

use crate::{SeoError, SeoSite, UrlPath};

/// The most addresses one `IndexNow` request may carry.
const MAX_URLS: usize = 10_000;

/// `IndexNow` for one site: where its key file lives, where requests go, and
/// which addresses it may send.
///
/// The key file sits in the blog's own folder, not at the root of the
/// address, so a blog mounted under another site needs nothing from that
/// site. Every request names the file in `keyLocation`, and search engines
/// accept only addresses inside the folder that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexNow {
    key: IndexNowKey,
    endpoint: AbsoluteUrl,
    host: String,
    scope: String,
    key_path: UrlPath,
    key_location: AbsoluteUrl,
}

impl IndexNow {
    /// `IndexNow` for `site` with the key and endpoint from `settings`.
    ///
    /// # Errors
    ///
    /// [`SeoError::Path`] or [`SeoError::Url`] when the key file's address
    /// cannot be built from the site's address and mount folder.
    pub fn new(site: &SeoSite, settings: &IndexNowSettings) -> Result<Self, SeoError> {
        let key = settings.key.clone();
        let endpoint = match &settings.endpoint {
            Some(endpoint) => endpoint.clone(),
            None => parse(IndexNowSettings::SHARED_ENDPOINT)?,
        };
        let scope = site
            .url_for(&site.default_language, &UrlPath::root())?
            .to_string();
        let key_path = UrlPath::try_new(&format!("/{key}.txt"))?;
        let key_location = site.url_for(&site.default_language, &key_path)?;
        Ok(Self {
            key,
            endpoint,
            host: site.address.host().to_owned(),
            scope,
            key_path,
            key_location,
        })
    }

    /// Where the requests are sent.
    pub fn endpoint(&self) -> &AbsoluteUrl {
        &self.endpoint
    }

    /// The full address of the key file, sent as `keyLocation`.
    pub fn key_location(&self) -> &AbsoluteUrl {
        &self.key_location
    }

    /// The key file the site must serve, so search engines can check that
    /// the addresses they are sent belong to it.
    pub fn key_file(&self) -> KeyFile {
        KeyFile {
            path: self.key_path.clone(),
            body: self.key.to_string(),
        }
    }

    /// The requests that announce `changed`: each address once, in the order
    /// first given, at most 10,000 to a request. Nothing changed gives no
    /// request.
    ///
    /// # Errors
    ///
    /// [`SeoError::IndexNowScope`] for the first address outside the folder
    /// that holds the key file, because search engines refuse the whole
    /// request when one address is outside it.
    pub fn submissions(&self, changed: &[AbsoluteUrl]) -> Result<Vec<Submission>, SeoError> {
        let mut seen = BTreeSet::new();
        let mut unique = Vec::with_capacity(changed.len());
        for url in changed {
            if !url.as_str().starts_with(&self.scope) {
                return Err(SeoError::IndexNowScope {
                    url: url.to_string(),
                    scope: self.scope.clone(),
                });
            }
            if seen.insert(url.as_str()) {
                unique.push(url.clone());
            }
        }
        Ok(unique
            .chunks(MAX_URLS)
            .map(|urls| self.submission(urls))
            .collect())
    }

    /// One request for `urls`.
    fn submission(&self, urls: &[AbsoluteUrl]) -> Submission {
        let list: Vec<&str> = urls.iter().map(AbsoluteUrl::as_str).collect();
        let body = json!({
            "host": self.host,
            "key": self.key.as_str(),
            "keyLocation": self.key_location.as_str(),
            "urlList": list,
        });
        Submission {
            urls: urls.to_vec(),
            body: body.to_string().into_bytes(),
        }
    }
}

/// The `IndexNow` key file: a text file that holds only the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFile {
    path: UrlPath,
    body: String,
}

impl KeyFile {
    /// Where the file is served, inside the mount folder like every page,
    /// such as `/3f9c0b5e2a8d4c71.txt`.
    pub fn path(&self) -> &UrlPath {
        &self.path
    }

    /// The text the file holds: the key.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// One `IndexNow` request: the addresses it announces and its JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    urls: Vec<AbsoluteUrl>,
    body: Vec<u8>,
}

impl Submission {
    /// The addresses this request announces.
    pub fn urls(&self) -> &[AbsoluteUrl] {
        &self.urls
    }

    /// The JSON body, ready to send.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

/// Parses a constant address.
fn parse(text: &str) -> Result<AbsoluteUrl, SeoError> {
    AbsoluteUrl::parse(text).map_err(|source| SeoError::Url {
        path: text.to_owned(),
        source,
    })
}
