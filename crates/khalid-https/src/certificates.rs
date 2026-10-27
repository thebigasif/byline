//! The certificate the server answers with, shared between the listener
//! and whatever renews it.

use std::sync::{Arc, PoisonError, RwLock};

use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

use crate::challenges::ACME_TLS;
use crate::{Certificate, Challenges};

/// The certificate the TLS listener answers with. A new one, such as a
/// renewal, takes effect from the next connection, without a restart.
///
/// While Let's Encrypt checks the domain over TLS-ALPN-01, its check gets
/// the answer from [`Certificates::challenges`] instead, and every other
/// visitor still gets the site's certificate.
///
/// It is cheap to clone, and every clone sees the same certificate.
#[derive(Debug, Clone, Default)]
pub struct Certificates {
    current: Arc<RwLock<Option<Certificate>>>,
    challenges: Challenges,
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

    /// The answers to the checks Let's Encrypt is making, shared with the
    /// port 80 router.
    pub fn challenges(&self) -> &Challenges {
        &self.challenges
    }
}

impl ResolvesServerCert for Certificates {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let is_check = hello
            .alpn()
            .is_some_and(|mut offered| offered.any(|protocol| protocol == ACME_TLS));
        if is_check {
            return hello
                .server_name()
                .and_then(|name| self.challenges.tls_answer(name));
        }
        self.current()
            .map(|certificate| certificate.certified_key())
    }
}
