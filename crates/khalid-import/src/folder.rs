//! A folder of markdown files, as Hugo, Jekyll, Astro and Eleventy keep
//! a blog: one file per post with front matter on top.

mod export;
mod read;

pub use export::{Export, Front, SourceFile, UnreadFile};
pub use read::{FileError, FolderError, read_folder};
