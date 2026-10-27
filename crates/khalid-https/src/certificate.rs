//! A certificate chain and its private key, read from PEM, ready to serve.

use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rustls::crypto::ring::sign::any_supported_type;
use rustls::sign::CertifiedKey;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use x509_parser::error::X509Error;
use x509_parser::extensions::GeneralName;
use x509_parser::nom;

use crate::HttpsError;

/// A certificate the server can answer with: its chain, its key, the names
/// it covers and when it is valid.
///
/// It is cheap to clone. The key is never printed: `Debug` shows only the
/// names and dates.
#[derive(Clone)]
pub struct Certificate {
    key: Arc<CertifiedKey>,
    names: Vec<String>,
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
}

impl Certificate {
    /// Reads a certificate from its PEM chain, the site's own certificate
    /// first, and its PEM private key.
    ///
    /// # Errors
    ///
    /// Returns [`HttpsError::Pem`] when either is not PEM,
    /// [`HttpsError::NoCertificate`] when the chain is empty,
    /// [`HttpsError::Certificate`] when the first certificate cannot be
    /// read, and [`HttpsError::Key`] when TLS cannot sign with the key.
    pub fn from_pem(chain: &[u8], key: &[u8]) -> Result<Self, HttpsError> {
        let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(chain)
            .collect::<Result<_, _>>()
            .map_err(|source| HttpsError::Pem { source })?;
        let first = chain.first().ok_or(HttpsError::NoCertificate)?;
        let (_, parsed) = x509_parser::parse_x509_certificate(first).map_err(|error| {
            let source = match error {
                nom::Err::Error(source) | nom::Err::Failure(source) => source,
                nom::Err::Incomplete(_) => X509Error::InvalidCertificate,
            };
            HttpsError::Certificate { source }
        })?;
        let validity = parsed.validity();
        let time = |seconds| {
            DateTime::from_timestamp(seconds, 0).ok_or(HttpsError::Certificate {
                source: X509Error::InvalidDate,
            })
        };
        let not_before = time(validity.not_before.timestamp())?;
        let not_after = time(validity.not_after.timestamp())?;
        let names = parsed
            .subject_alternative_name()
            .map_err(|source| HttpsError::Certificate { source })?
            .map(|names| {
                names
                    .value
                    .general_names
                    .iter()
                    .filter_map(|name| match name {
                        GeneralName::DNSName(dns) => Some(dns.to_ascii_lowercase()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let key =
            PrivateKeyDer::from_pem_slice(key).map_err(|source| HttpsError::Pem { source })?;
        let signing = any_supported_type(&key).map_err(|source| HttpsError::Key { source })?;
        Ok(Self {
            key: Arc::new(CertifiedKey::new(chain, signing)),
            names,
            not_before,
            not_after,
        })
    }

    /// Whether the certificate names `domain`, such as `blog.example.com`.
    pub fn covers(&self, domain: &str) -> bool {
        self.names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(domain))
    }

    /// The last moment the certificate is valid.
    pub fn not_after(&self) -> DateTime<Utc> {
        self.not_after
    }

    /// When to get the next certificate: once two thirds of this one's life
    /// have passed, as Let's Encrypt advises, so a 90-day certificate
    /// renews with 30 days left and a shorter one keeps the same share.
    pub fn renew_at(&self) -> DateTime<Utc> {
        let life = self.not_after - self.not_before;
        self.not_before + life * 2 / 3
    }

    /// The chain and signing key as TLS uses them.
    pub(crate) fn certified_key(&self) -> Arc<CertifiedKey> {
        Arc::clone(&self.key)
    }
}

impl fmt::Debug for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Certificate")
            .field("names", &self.names)
            .field("not_before", &self.not_before)
            .field("not_after", &self.not_after)
            .finish_non_exhaustive()
    }
}
