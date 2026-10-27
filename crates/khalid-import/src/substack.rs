//! Substack: the archive that Settings, Exports gives, with `posts.csv`
//! listing every post and one HTML file per post in its `posts/` folder.

mod body;
mod export;
mod plan;
mod read;
mod redirect_map;
mod report;

pub use crate::csv::CsvError;
pub use body::BodyError;
pub use export::{Audience, Export, Post, PostKind, UnreadRow};
pub use plan::{FailedPost, ImportedPost, LeftPost, LeftReason, Plan, PostError, plan};
pub use read::{ExportError, RowError, read_export};
pub use redirect_map::{MapEntry, REDIRECTS_CSV};
pub use report::Report;
