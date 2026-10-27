//! An account with a certificate authority over ACME (RFC 8555), such as
//! Let's Encrypt.

use std::fmt;

use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, OrderStatus, RetryPolicy,
};
use khalid_outbound::Outbound;

use crate::acme_http::Through;
use crate::challenges::DIGEST_LENGTH;
use crate::{Challenges, Domain, HttpsError};

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

    /// Gets a certificate for `domain`, answering the authority's check
    /// through `challenges` with the first of `checks` it offers. The
    /// answers are cleared afterwards, whether or not it worked.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::NoCheck`] when the authority offers none of
    /// `checks`, [`HttpsError::Refused`] when it does not accept the
    /// answer, and [`HttpsError::Acme`] when it cannot be reached.
    pub async fn issue(
        &self,
        domain: &Domain,
        challenges: &Challenges,
        checks: &[Check],
    ) -> Result<Issued, HttpsError> {
        let issued = self.order(domain, challenges, checks).await;
        challenges.clear();
        issued
    }

    async fn order(
        &self,
        domain: &Domain,
        challenges: &Challenges,
        checks: &[Check],
    ) -> Result<Issued, HttpsError> {
        let acme = |source| HttpsError::Acme { source };
        let identifiers = [Identifier::Dns(domain.as_str().to_owned())];
        let mut order = self
            .account
            .new_order(&NewOrder::new(&identifiers))
            .await
            .map_err(acme)?;
        let mut authorizations = order.authorizations();
        while let Some(authorization) = authorizations.next().await {
            let mut authorization = authorization.map_err(acme)?;
            if authorization.status == AuthorizationStatus::Valid {
                continue;
            }
            let offered = |check: &Check| {
                let wanted = check.challenge_type();
                authorization.challenges.iter().any(|c| c.r#type == wanted)
            };
            let check = checks
                .iter()
                .copied()
                .find(offered)
                .ok_or(HttpsError::NoCheck)?;
            let mut challenge = authorization
                .challenge(check.challenge_type())
                .ok_or(HttpsError::NoCheck)?;
            let answer = challenge.key_authorization();
            match check {
                Check::TlsAlpn => {
                    let digest = <[u8; DIGEST_LENGTH]>::try_from(answer.digest().as_ref())
                        .map_err(|_| acme(instant_acme::Error::Str("a digest is not 32 bytes")))?;
                    challenges.answer_tls(domain.as_str(), &digest)?;
                }
                Check::Http => challenges.answer_http(&challenge.token, answer.as_str()),
            }
            challenge.set_ready().await.map_err(acme)?;
        }
        match order.poll_ready(&RetryPolicy::default()).await {
            Ok(OrderStatus::Ready) => {}
            Ok(_) => return Err(HttpsError::Refused { detail: None }),
            Err(instant_acme::Error::Api(problem)) => {
                return Err(HttpsError::Refused {
                    detail: problem.detail,
                });
            }
            Err(source) => return Err(acme(source)),
        }
        let key = order.finalize().await.map_err(acme)?;
        let chain = order
            .poll_certificate(&RetryPolicy::default())
            .await
            .map_err(acme)?;
        Ok(Issued { chain, key })
    }
}

/// How a certificate authority checks that this server answers for the
/// domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Check {
    /// A one-off certificate in the TLS handshake on port 443 (TLS-ALPN-01).
    TlsAlpn,
    /// A file at `/.well-known/acme-challenge/` on port 80 (HTTP-01).
    Http,
}

impl Check {
    fn challenge_type(self) -> ChallengeType {
        match self {
            Self::TlsAlpn => ChallengeType::TlsAlpn01,
            Self::Http => ChallengeType::Http01,
        }
    }
}

/// A new certificate as PEM: the chain, then its key. `Debug` shows neither.
#[derive(Clone, PartialEq, Eq)]
pub struct Issued {
    /// The chain, the site's certificate first.
    pub chain: String,
    /// The private key. A secret: never print it.
    pub key: String,
}

impl fmt::Debug for Issued {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Issued").finish_non_exhaustive()
    }
}

impl fmt::Debug for Acme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Acme")
            .field("id", &self.account.id())
            .finish_non_exhaustive()
    }
}
