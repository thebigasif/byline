//! Why HTTPS could not be set up.

/// Why a certificate could not be read or the TLS listener set up.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HttpsError {
    /// The certificate or key file is not PEM.
    #[error("the certificate or its key is not in PEM form")]
    Pem {
        /// What the PEM reader reported.
        #[source]
        source: rustls_pki_types::pem::Error,
    },
    /// The PEM holds no certificate.
    #[error("the certificate file holds no certificate")]
    NoCertificate,
    /// The certificate cannot be read as X.509.
    #[error("the certificate cannot be read")]
    Certificate {
        /// What the X.509 reader reported.
        #[source]
        source: x509_parser::error::X509Error,
    },
    /// The key is of a kind TLS cannot sign with.
    #[error("the private key cannot be used for TLS")]
    Key {
        /// What the TLS library reported.
        #[source]
        source: rustls::Error,
    },
    /// The certificate answering a TLS-ALPN-01 check could not be made.
    #[error("the answer to Let's Encrypt's TLS check could not be made")]
    Challenge {
        /// What the certificate maker reported.
        #[source]
        source: rcgen::Error,
    },
    /// The certificate authority could not be reached, or refused.
    #[error("the certificate authority could not be reached, or refused")]
    Acme {
        /// What the ACME client reported.
        #[source]
        source: instant_acme::Error,
    },
    /// A saved account key cannot be read or written.
    #[error("the saved account key cannot be read")]
    AccountFile {
        /// What the JSON reader reported.
        #[source]
        source: serde_json::Error,
    },
    /// The certificate authority offers none of the checks this server can
    /// answer.
    #[error("the certificate authority offers no check this server can answer")]
    NoCheck,
    /// The certificate authority did not accept the answer to its check.
    #[error("the certificate authority did not accept the answer to its check")]
    Refused {
        /// What the authority said, if it said anything.
        detail: Option<String>,
    },
    /// A new account is needed, and its owner has not agreed to the
    /// certificate authority's terms.
    #[error("a new account needs its owner's yes to the certificate authority's terms")]
    TermsNotAgreed,
    /// A file in the HTTPS folder cannot be read or written.
    #[error("{} cannot be read or written", path.display())]
    Folder {
        /// The file or folder.
        path: std::path::PathBuf,
        /// What the system reported.
        #[source]
        source: std::io::Error,
    },
    /// The TLS settings could not be built.
    #[error("TLS could not be set up")]
    Tls {
        /// What the TLS library reported.
        #[source]
        source: rustls::Error,
    },
}
