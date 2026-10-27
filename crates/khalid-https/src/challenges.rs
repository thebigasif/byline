//! The answers to the checks Let's Encrypt makes before it issues a
//! certificate: a file on port 80 (HTTP-01), or a special certificate in
//! the TLS handshake on port 443 (TLS-ALPN-01).

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use rcgen::{CertificateParams, CustomExtension, KeyPair};
use rustls::crypto::ring::sign::any_supported_type;
use rustls::sign::CertifiedKey;
use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::HttpsError;

/// The length of the SHA-256 digest a TLS-ALPN-01 certificate carries.
pub(crate) const DIGEST_LENGTH: usize = 32;

/// The protocol a TLS-ALPN-01 check offers in its handshake (RFC 8737).
pub(crate) const ACME_TLS: &[u8] = b"acme-tls/1";

/// The answers to the checks Let's Encrypt is making right now, read by
/// the port 80 router and the TLS listener while an order is open.
///
/// It is cheap to clone, and every clone sees the same answers.
#[derive(Debug, Clone, Default)]
pub struct Challenges {
    answers: Arc<RwLock<Answers>>,
}

/// The answers, by token for HTTP-01 and by domain for TLS-ALPN-01.
#[derive(Debug, Default)]
struct Answers {
    http: BTreeMap<String, String>,
    tls: BTreeMap<String, Arc<CertifiedKey>>,
}

impl Challenges {
    /// No answers yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The text to answer `/.well-known/acme-challenge/<token>` with, if
    /// that check is open.
    pub fn http_answer(&self, token: &str) -> Option<String> {
        self.read().http.get(token).cloned()
    }

    /// The certificate to answer a TLS-ALPN-01 check for `domain` with, if
    /// that check is open.
    pub(crate) fn tls_answer(&self, domain: &str) -> Option<Arc<CertifiedKey>> {
        self.read().tls.get(&domain.to_ascii_lowercase()).cloned()
    }

    /// Answers the HTTP-01 check for `token` with `key_authorization`.
    pub fn answer_http(&self, token: &str, key_authorization: &str) {
        self.write()
            .http
            .insert(token.to_owned(), key_authorization.to_owned());
    }

    /// Answers the TLS-ALPN-01 check for `domain` with a fresh
    /// self-signed certificate carrying `digest`, the SHA-256 of the key
    /// authorization (RFC 8737).
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Challenge`] when the certificate cannot be
    /// made, and [`HttpsError::Key`] when TLS cannot sign with its key.
    pub fn answer_tls(&self, domain: &str, digest: &[u8; DIGEST_LENGTH]) -> Result<(), HttpsError> {
        let domain = domain.to_ascii_lowercase();
        let made = || -> Result<_, rcgen::Error> {
            let mut params = CertificateParams::new(vec![domain.clone()])?;
            params.custom_extensions = vec![CustomExtension::new_acme_identifier(digest)];
            let key = KeyPair::generate()?;
            let certificate = params.self_signed(&key)?;
            Ok((certificate.der().clone(), key.serialize_der()))
        };
        let (der, key) = made().map_err(|source| HttpsError::Challenge { source })?;
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key));
        let signing = any_supported_type(&key).map_err(|source| HttpsError::Key { source })?;
        self.write()
            .tls
            .insert(domain, Arc::new(CertifiedKey::new(vec![der], signing)));
        Ok(())
    }

    /// Forgets every answer, once an order is done.
    pub fn clear(&self) {
        let mut answers = self.write();
        answers.http.clear();
        answers.tls.clear();
    }

    fn read(&self) -> RwLockReadGuard<'_, Answers> {
        self.answers.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, Answers> {
        self.answers.write().unwrap_or_else(PoisonError::into_inner)
    }
}
