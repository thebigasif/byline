//! A folder of markdown files, as Hugo, Jekyll, Astro and Eleventy keep
//! a blog: one file per post with front matter on top.

mod body;
mod export;
mod plan;
mod read;

pub use body::BodyError;
pub use export::{Export, Front, SourceFile, UnreadFile};
pub use plan::{FailedPost, ImportedPost, Plan, PostError, plan};
pub use read::{FileError, FolderError, read_folder};
