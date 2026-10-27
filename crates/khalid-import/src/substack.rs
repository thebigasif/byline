//! Substack: the archive that Settings, Exports gives, with `posts.csv`
//! listing every post and one HTML file per post in its `posts/` folder.

mod export;
mod read;

pub use crate::csv::CsvError;
pub use export::{Audience, Export, Post, PostKind, UnreadRow};
pub use read::{ExportError, RowError, read_export};
