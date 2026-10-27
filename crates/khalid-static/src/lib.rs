//! Static mode (`mode = "static"`, KH-OPS-1): the features a static site
//! cannot have, and its pages with every form that would send to the site
//! itself replaced by a short note for the reader.
//!
//! A static host only sends files, so nothing answers a form, a search or a
//! comment. This crate names those features. It knows nothing of templates,
//! the build or the messages the owner reads: the binary makes those from
//! [`ServerFeature`].

mod server_feature;

pub use server_feature::ServerFeature;
