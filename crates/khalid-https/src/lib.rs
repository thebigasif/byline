//! HTTPS with the site's own certificate (plan section 4.7, KH-INS-4).
//!
//! This crate owns the TLS side of `khalid serve --domain`: the domain and
//! its certificate read from PEM, when to renew it, the shared set of
//! certificates the server answers with, the listener that turns each
//! connection into a TLS one, and the answers to Let's Encrypt's checks on
//! port 80 and in the TLS handshake, and the account with Let's Encrypt,
//! spoken to through the one outbound client. It must not know about
//! pages, content or the database.

mod acme;
mod acme_http;
mod certificate;
mod certificates;
mod challenges;
mod domain;
mod error;
mod listener;
mod port_80;
mod renew;
mod store;

pub use acme::{AccountKey, Acme, AgreedToTerms, Check, Directory, Issued, LETS_ENCRYPT_TERMS};
pub use certificate::Certificate;
pub use certificates::Certificates;
pub use challenges::Challenges;
pub use domain::{Domain, DomainError};
pub use error::HttpsError;
pub use listener::TlsListener;
pub use port_80::port_80;
pub use renew::{Next, Renewal, next};
pub use store::HttpsFolder;
