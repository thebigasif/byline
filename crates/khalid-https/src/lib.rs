//! HTTPS with the site's own certificate (plan section 4.7, KH-INS-4).
//!
//! This crate owns the TLS side of `khalid serve --domain`: the certificate
//! and its key read from PEM, when to renew it, and the shared set of
//! certificates the server answers with. It must not know about pages,
//! content or the database.

mod certificate;
mod certificates;
mod error;

pub use certificate::Certificate;
pub use certificates::Certificates;
pub use error::HttpsError;
