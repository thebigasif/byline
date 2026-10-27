//! Images for the public site, made once at publish and never at request.
//!
//! This crate owns turning one original image into the sizes and formats a
//! page offers: WebP in a responsive set of widths, plus the original's
//! format for older browsers. AVIF joins when its encoder passes the
//! dependency policy. Every derived file is named by a
//! hash of the original's bytes and kept in the site's `data/media/`
//! folder, so a file that did not change is never encoded twice (survey
//! lesson 5, RULES.md BL-PERF-5).
//!
//! It must not know about HTTP routes, templates or the content tree. The
//! render crate shows what this crate made; the wiring code connects them.

mod content_key;
mod derive;
mod media_cache;
mod media_error;
mod media_type;
mod original;
mod variant;

pub use content_key::ContentKey;
pub use derive::{DerivedFile, DerivedImage, derive, derive_in_background};
pub use media_cache::MediaCache;
pub use media_error::MediaError;
pub use media_type::{Kind, MediaType, ParseMediaTypeError, Refusal};
pub use original::Original;
pub use variant::{Format, Variant, widths};
