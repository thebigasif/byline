//! Bringing a blog from another system into Khalid (plan section 5.7, PRD KH-API-7).
//!
//! This crate owns reading other systems' exports and turning them into
//! Khalid content files, with kept slugs, a redirect map and a report of what
//! came in and what did not. One module per source: `wordpress`, `ghost`,
//! `medium`, `substack` and `folder`, for a folder of markdown files.
//! It must not know about rendering, HTTP or the database. It reads text it
//! is given and never fetches anything itself. Every file it plans is read
//! back with `khalid-content` first, so an import never writes a file the
//! site cannot read.

mod csv;
mod find;
pub mod folder;
mod front_matter;
pub mod ghost;
mod markdown;
pub mod medium;
mod pieces;
mod redirects;
mod sentences;
mod slug;
pub mod substack;
pub mod wordpress;
mod write;

pub use redirects::{MapTarget, Matching, MoveStatus};
pub use slug::SlugOrigin;
pub use write::{Place, PlannedFile, WriteError};
