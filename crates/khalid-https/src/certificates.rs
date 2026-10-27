//! The certificate the server answers with, shared between the listener
//! and whatever renews it.

use std::sync::{Arc, PoisonError, RwLock};

use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

use crate::Certificate;

/// The certificate the TLS listener answers with. A new one, such as a
/// renewal, takes effect from the next connection, without a restart.
///
/// It is cheap to clone, and every clone sees the same certificate.
#[derive(Debug, Clone, Default)]
pub struct Certificates {
    current: Arc<RwLock<Option<Certificate>>>,
}

impl Certificates {
    /// No certificate yet. Connections fail their handshake until one is
    /// set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answers every connection from now on with `certificate`.
    pub fn set(&self, certificate: Certificate) {
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Some(certificate);
    }

    /// The certificate connections are answered with, if there is one.
    pub fn current(&self) -> Option<Certificate> {
        self.current
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ResolvesServerCert for Certificates {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.current()
            .map(|certificate| certificate.certified_key())
    }
}
