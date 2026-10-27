//! An account with a certificate authority over ACME (RFC 8555), such as
//! Let's Encrypt.

use std::fmt;

use instant_acme::{Account, AccountCredentials, NewAccount};
use khalid_outbound::Outbound;

use crate::HttpsError;
use crate::acme_http::Through;

/// The address of Let's Encrypt's subscriber agreement, which a new account
/// agrees to.
pub const LETS_ENCRYPT_TERMS: &str = "https://letsencrypt.org/repository/";

/// Where certificates come from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Directory {
    /// Let's Encrypt, for certificates browsers trust.
    LetsEncrypt,
    /// Let's Encrypt's staging service, for trying a setup without its
    /// rate limits. Browsers do not trust its certificates.
    LetsEncryptStaging,
    /// Another ACME service at this directory address, such as a test one.
    Other(String),
}

impl Directory {
    /// The directory address.
    pub fn url(&self) -> &str {
        match self {
            Self::LetsEncrypt => instant_acme::LetsEncrypt::Production.url(),
            Self::LetsEncryptStaging => instant_acme::LetsEncrypt::Staging.url(),
            Self::Other(url) => url,
        }
    }
}

/// The site owner's yes to the certificate authority's terms. Only code
/// that asked the owner, or was told with `--accept-terms`, makes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgreedToTerms;

/// The account's private key and address, to keep and sign in with next
/// time. It is a secret: `Debug` never shows it.
pub struct AccountKey(AccountCredentials);

impl AccountKey {
    /// Reads a key saved with [`AccountKey::to_json`].
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::AccountFile`] when `json` is not a saved key.
    pub fn from_json(json: &[u8]) -> Result<Self, HttpsError> {
        serde_json::from_slice(json)
            .map(Self)
            .map_err(|source| HttpsError::AccountFile { source })
    }

    /// The key as JSON, to save where only the site's owner can read it.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::AccountFile`] when it cannot be written out.
    pub fn to_json(&self) -> Result<Vec<u8>, HttpsError> {
        serde_json::to_vec(&self.0).map_err(|source| HttpsError::AccountFile { source })
    }
}

impl fmt::Debug for AccountKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccountKey(hidden)")
    }
}

/// An account with a certificate authority, spoken to through the one
/// outbound client (RULES.md BL-RUN-8).
pub struct Acme {
    account: Account,
}

impl Acme {
    /// Opens a new account at `directory`, agreeing to its terms, and
    /// returns it with the key to sign in with next time.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Acme`] when the service cannot be reached or
    /// refuses.
    pub async fn sign_up(
        outbound: &Outbound,
        directory: &Directory,
        _agreed: AgreedToTerms,
    ) -> Result<(Self, AccountKey), HttpsError> {
        let new = NewAccount {
            contact: &[],
            terms_of_service_agreed: true,
            only_return_existing: false,
        };
        let (account, key) = Account::builder_with_http(Through::boxed(outbound))
            .create(&new, directory.url().to_owned(), None)
            .await
            .map_err(|source| HttpsError::Acme { source })?;
        Ok((Self { account }, AccountKey(key)))
    }

    /// Opens the account `key` belongs to.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Acme`] when the service cannot be reached or
    /// refuses.
    pub async fn sign_in(outbound: &Outbound, key: AccountKey) -> Result<Self, HttpsError> {
        let account = Account::builder_with_http(Through::boxed(outbound))
            .from_credentials(key.0)
            .await
            .map_err(|source| HttpsError::Acme { source })?;
        Ok(Self { account })
    }

    /// The account's address at the certificate authority.
    pub fn id(&self) -> &str {
        self.account.id()
    }
}

impl fmt::Debug for Acme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Acme")
            .field("id", &self.account.id())
            .finish_non_exhaustive()
    }
}
