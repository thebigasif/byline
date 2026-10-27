//! HTTPS with the site's own certificate (plan section 4.7, KH-INS-4).
//!
//! This crate owns the TLS side of `khalid serve --domain`: the certificate
//! and its key read from PEM, when to renew it, the shared set of
//! certificates the server answers with, and the listener that turns each
//! connection into a TLS one. It must not know about pages, content or the
//! database.

mod certificate;
mod certificates;
mod error;
mod listener;

pub use certificate::Certificate;
pub use certificates::Certificates;
pub use error::HttpsError;
pub use listener::TlsListener;
