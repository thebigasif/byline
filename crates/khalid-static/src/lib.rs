//! Static mode (`mode = "static"`, KH-OPS-1): the features a static site
//! cannot have, and its pages with every form that would send to the site
//! itself replaced by a short note for the reader.
//!
//! A static host only sends files, so nothing answers a form, a search or a
//! comment. This crate names those features and finds the forms on a
//! rendered page. It knows nothing of templates, the build or the messages
//! the owner reads: the binary makes those from [`ServerFeature`].

mod lock_error;
mod page_form;
mod server_feature;

pub use lock_error::LockError;
pub use page_form::{FormKind, PageForm, forms};
pub use server_feature::ServerFeature;
