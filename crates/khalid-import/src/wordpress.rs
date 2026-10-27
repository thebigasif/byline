//! `WordPress`: the WXR export file that Tools, Export writes.

mod export;
mod wxr;
mod wxr_error;

pub use export::{Author, Export, Site, Term};
pub use wxr::read_wxr;
pub use wxr_error::WxrError;
