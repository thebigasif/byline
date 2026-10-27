//! Substack: the archive that Settings, Exports gives, with `posts.csv`
//! listing every post and one HTML file per post in its `posts/` folder.

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the Substack plan converts bodies, in the next part"
    )
)]
mod body;
mod export;
mod read;

pub use crate::csv::CsvError;
pub use body::BodyError;
pub use export::{Audience, Export, Post, PostKind, UnreadRow};
pub use read::{ExportError, RowError, read_export};
